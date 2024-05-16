# chapter3练习

## 功能实现概述

本次实验实现了获取任务信息 `sys_task_info` ，其实现原理是通过向 `TaskControlBlock` 中添加 `task_info` 和 `start_time` 字段，获取和查询当前任务信息。其中 `task_info` 为 `TaskInfo` 结构体,其中包含需要获取的任务的相关信息， `start_time` 表示的是一个任务第一次被调度时刻，用于获取任务的周转时间 `time` 。 `status` 可以直接设为 `Running`,  `syscall_times` 在每次系统调用前（即`syscall`函数前）重新计数。

## 简答作业

### 第 1 题

程序出错行为：
```
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003ac, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
[kernel] IllegalInstruction in application, kernel killed it.
```

rustsbi 及其版本：

```
[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0
```

### 第 2 题

1. `a0`保存系统调用的返回值，代表指向分配 Trap 上下文之后的内核栈栈顶，`__restore`用于在初始化完成后或切换到下一个应用程序时恢复 Trap 上下文。
2.  这几行汇编代码特殊处理了 `sstatus` `sepc` `sscratch` 这三个寄存器，其中 `sstatus` 给出用户态的特权级的信息， `sepc` 记录在用户态发生Trap之前执行的最后一条指令的地址， `sscratch` 用于保存用户栈的地址。
3.  `x2`是当前栈的栈指针，用户栈的栈指针被保存在了`sscratch`中， `x4`除非我们手动出于一些特殊用途使用它，否则一般也不会被用到。
4.  该指令之后，`sp`指向用户栈， `sscratch`指向内核栈。
5.  发生状态切换的是 `csrrw sp, sscratch, sp`, 因为在这一行中，csrrw 指令被用来交换 sp 寄存器（堆栈指针）的值与 sscratch 寄存器的值，这样就将 sp 指向了用户堆栈，而 sscratch 指向了内核堆栈。这个操作使得处理器的执行环境从内核态切换到了用户态，也就是从 S 态切换到了 U 态。
6.  该指令之后，`sp`指向内核栈， `sscratch`指向用户栈。
7.  从 U 态（User Mode）进入 S 态（Supervisor Mode）是在第 9 行的 csrrw 指令发生的。
   
## 荣誉守则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    暂无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    暂无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
