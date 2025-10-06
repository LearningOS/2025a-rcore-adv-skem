[rustsbi] Implementation     : RustSBI-QEMU Version 0.2.0-alpha.2

[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003a4, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.


二者呈现一个对称特点。
__alltraps：
当 CPU 从 U 态（用户态）进入 S 态（内核态） 触发 trap 时执行。
它完成：保存用户态寄存器上下文（包括 sstatus、sepc、sp 等）；将用户栈指针保存到 sscratch；构造一个 TrapContext；调用 Rust 层的 trap_handler()。

__restore：
当内核要从 S 态恢复执行 U 态程序 时执行。
它完成：从 TrapContext 中恢复用户寄存器；恢复 sstatus、sepc；切换回用户栈；执行 sret 回到用户态。

L40：刚进入 __restore 时，sp 代表什么值？
__restore:此时的 sp 指向 内核栈上的 TrapContext 的起始地址。在进入 trap 时，__alltraps 用
addi sp, sp, -34*8
为 TrapContext 预留了空间，所以现在 sp 仍指向这个结构


__restore 的两种使用情景系统调用 / 异常返回用户态用户态程序执行 ecall 或触发异常；内核处理完后，通过 __restore 恢复用户态上下文；执行 sret 回到用户程序继续运行。任务切换（上下文切换）当前任务陷入内核，保存其上下文；调度器选择下一个任务；通过 __restore 恢复新任务的 TrapContext；切换回该任务的用户栈与状态。


L43-L48：这几行汇编代码特殊处理了哪些寄存器？它们的意义是什么？
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2

指令	寄存器	含义
ld t0, 32*8(sp)	从 TrapContext 取出保存的 sstatus	
ld t1, 33*8(sp)	从 TrapContext 取出保存的 sepc	
ld t2, 2*8(sp)	从 TrapContext 取出保存的用户栈指针 sp	

然后分别写入：

csrw sstatus, t0：恢复用户态的 sstatus，其中最关键的是 SPP=0（用户态），SPIE=1（允许中断），表示下次执行 sret 时切换回 U 模式。

csrw sepc, t1：恢复用户态返回地址，sret 会跳转到这里执行用户代码。

csrw sscratch, t2：恢复用户态的 sp，保存到 sscratch，稍后会在 csrrw sp, sscratch, sp 时交换回来。


L50-L56：为何跳过 x2 和 x4？
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr


x2 = sp（栈指针）：
不直接从 TrapContext 恢复，因为稍后 csrrw sp, sscratch, sp 会通过交换恢复真正的用户栈指针。

x4 = tp（线程指针）：
通常由硬件线程管理，不由应用直接使用，因此系统保留或固定不动。

L60：该指令之后，sp 和 sscratch 中的值分别有什么意义？
csrrw sp, sscratch, sp


执行后：

sp ← 原来 sscratch 的值（即用户态的栈指针）；

sscratch ← 原来 sp 的值（即内核态栈顶地址）。



此时 sp 已恢复为用户栈；

sscratch 保存了内核栈顶地址，方便下次陷入 trap 时使用。

__restore 中发生状态切换的指令是哪一条？为何执行后进入用户态？

状态切换在：

sret


执行 sret 后：

硬件根据 sstatus 寄存器的 SPP 位（Supervisor Previous Privilege）决定返回的特权级；

若 SPP=0 → 切换到 U 模式；

若 SPP=1 → 切换回 S 模式。

同时 PC ← sepc，CPU 从用户程序的下一条指令开始执行。

L13：该指令之后，sp 和 sscratch 中的值分别有什么意义？

（在 __alltraps 开头）

csrrw sp, sscratch, sp


执行后：

sp ← 原本用户栈指针保存在 sscratch 里的值；

sscratch ← 原本内核栈指针；

也就是现在：

sp 指向内核栈（准备保存 TrapContext）；

sscratch 保存用户栈指针（后面恢复时要用）。

从 U 态进入 S 态是哪一条指令发生的？

由硬件自动完成，当用户程序执行：

ecall


或发生异常/中断时，CPU 会：

自动保存当前 pc 到 sepc；

设置 SPP=0（记录来自 U 模式）；

切换到 S 模式；

跳转到 stvec 对应的入口函数（即 __alltraps）。



sstatus：定义返回后的特权级；
sepc：定义返回后的执行位置；
sscratch：临时存储用户态栈指针。


### 核心思想
需要什么数据，就扩展一个全局的表，存储我们需要的所有信息：内核状态表格化（Table-driven Kernel State）
- 表是什么结构？
- 表放在什么位置？
- 表需要什么接口？
- 管理表的数据竞争等等。（并发访问的安全性）

### syscall_count设计
任务调用次数统计（task_call_count）

系统调用计数表（syscall_count）

sys_trace 系统调用实现
支持 3 种操作模式：

读取指定地址的 u8 值；

向指定地址写入一个 u8 值；

查询当前任务某系统调用被调用的次数。

///下面是ai解释部分
🧩 一、这种思想从哪里来？

这种把信息集中存储在一张“表（table）”中的思想来源于：

🧠 操作系统内核中的结构化状态管理（State Table Model）

在早期的内核设计（如 UNIX、xv6）里，操作系统需要同时管理：

每个进程的寄存器状态；

每个进程的打开文件；

每个进程的系统调用记录；

每个进程的页表、调度状态等。

于是人们发现：

“内核的职责，就是维护所有进程的各种状态，而这些状态天然是离散、可枚举的——最自然的方式，就是表（table）。”

因此在 OS 内核里：

task_table[] 保存任务控制块；

syscall_table[] 保存系统调用分发表；

fd_table[] 保存文件描述符；

page_table[] 保存页表映射；

syscall_count[task][syscall] 保存行为统计。

你写的这个 syscall_count，正是这种思想的一个 微缩实现。

🧭 二、核心思想：状态集中、索引访问

它的核心优点是：

特点	含义
集中存储	把所有任务的系统调用记录放在一张二维表中，方便统一管理。
索引访问	用 task_id 和 syscall_id 作为行列索引，能 O(1) 快速访问。
数据隔离	每个任务只操作自己的一行，互不影响。
持久可查询	调度器或调试器可以随时读取表，获得系统运行统计。

例如：

syscall_count[task_id][syscall_id] += 1;


就是典型的 table-driven system accounting 模式。

⚙️ 三、它属于哪种思想范畴？

从体系结构上讲，它体现了两种经典思想：

Table-Driven Design（表驱动设计）

通过数组或映射表管理逻辑状态，而非一堆条件语句。

比如：syscall_table[] 存放每个系统调用的处理函数指针。

Monitor Pattern（监控模型）

系统通过一张表实时记录各类事件（调用次数、异常次数、页错误次数等），

后续调度器、性能分析器都可以基于这张表进行决策。

### 关注对称性
用户程序：
    count_syscall(id)
        ↓
    trace(Syscall, id, 0)
        ↓
    sys_trace(2, id, 0)     # 用户态
        ↓
    syscall(SYSCALL_TRACE, [2, id, 0])
        ↓  (ecall 陷入)
──────────── 内核分界 ────────────
trap_handler() 捕获 ecall
    ↓
syscall(syscall_id=410, args=[2, id, 0])   # 内核分发器
    ↓
sys_trace(trace_request=2, id, 0)          # 内核实现
    ↓
返回调用次数
──────────── 返回用户态 ────────────
count_syscall() 获得结果



# 荣誉准则 
在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    Brad群友，解释了自动评测怎么用

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    使用Chatgpt总结了一些设计思想

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。