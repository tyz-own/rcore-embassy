# chapter5练习

## 功能实现概述

在本次实验中，我根据实验要求实现了进程创建 `sys_spawn` 和stride调度算法以及 `sys_set_priority` 系统调用。
`sys_spawn`  实现过程中，我通过研读 `fork` 和 `exec` 相关代码并进行拼凑和微调最终得出了简易版的spawn代码。
`sys_set_priority` 的实现只需对参数进行排错并将参数用`BIG_STRIDE` 整除之后传给 `TaskControlBlockInner` 中新添的 `pass` 即可。
对于`stride` 调度算法的实现，需要在`TaskControlBlockInner` 中新添一个`stride`, 并将 `TaskManager` 中的 `fetch` 进行修改， 在准备队列中寻找stride最小的任务，并将其任务步幅进行更新并取出。


## 简答作业

### 第 1 题

不是，因为 `p2.stride` 溢出，导致数值从 260 变成了 5，其值比 `p1.stride` 小，因此依旧是 `p2` 执行。

### 第 2 题

假设STRIDE_MAX – STRIDE_MIN > BigStride / 2。因为 pass <= BigStride / 2,
所以pass < STRIDE_MAX – STRIDE_MIN, 则 STRIDE_MAX - pass > STRIDE_MIN, 即 stride 最大值减去步长值大于步长最小值，因此在 STRIDE_MAX 上一次被调度时 其值依旧大于STRIDE_MIN，因此不等式不成立。

代码如下：

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 > other.0 && other.0 + 0xFFFFFFFFFFFFFFFF - self.0 > BigStride / 2{
            Some(Ordering::Greater)
        } else if self.0 < other.0 && self.0 + 0xFFFFFFFFFFFFFFFF - other.0 > BigStride / 2{
            Some(Ordering::Less)
        } else if self.0 > other.0 {
            Some(Ordering::Greater)
        } else if self.0 < other.0 {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    
    }
}
```

## 荣誉守则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    暂无

2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    暂无

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。