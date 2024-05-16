# chapter3练习

## 功能实现概述

本次实验实现了获取任务信息 `sys_task_info` ，其实现原理是通过向 `TaskControlBlock` 中添加 `task_info` 和 `start_time` 字段，获取和查询当前任务信息。其中 `task_info` 为 `TaskInfo` 结构体,其中包含需要获取的任务的相关信息， `start_time` 表示的是一个任务第一次被调度时刻，用于获取任务的周转时间 `time` 。 `status` 可以直接设为 `Running`,  `syscall_times` 在每次系统调用前（即`syscall`函数前）重新计数。

## 简单作业

### 第 1 题

