# OS chapter8 实验报告

***2017011998 冯卓尔 计86***

---

### 分析

#### ch8_01.rs

现象：运行后报错`unwrap()`一个None的对象，OS崩溃退出。

解释：这是上课分析过的fork炸弹，一共会fork出`2^10`个进程。在进程fork过程中，操作系统的空间资源会迅速耗尽，`frame_alloc()`功能会无法得到有效内存，返回类型`Option<>`的内容是`None`因而不能够直接`unwrap()`。

解决办法：递归修改相应的`unwrap()`操作，先`if let Some(xxx) = xxxxOption`再进行下一步操作。即不再默认`frame_alloc()`能够一定成功分配内存。

#### ch8_02.rs

现象：运行后报错`unwrap()`一个None的对象，OS崩溃退出。

解释：这个测例与之前不同，是使用`mmap()`来耗尽内存资源的。同样地，当内存资源耗尽的时候，`frame_alloc()`函数无法分配内存，返回类型`Option<>`在未加检查的情况下被直接`unwrap()`从而导致运行时错误。

解决办法：同上，递归修改相应的`unwrap()`操作的时候，`mmap()`会用到`pagetable`的`push`方法，这个方法也不一定能够保证分配成功，因此需要递归的将底层`alloc`的结果传上来，再进行判断。先`if let Some(xxx) = xxxxOption`再进行下一步操作。即不再默认`memory_set.insert_framed_area()`能够一定成功分配内存。

#### ch8_03.rs

现象：非法指令错误

```
scause=0x2
[kernel] IllegalInstruction in application, core dumped.
Shell: Process 2 exited with code -3
```

解释：尝试进行非法内存映射（从0开始）。

解决办法：在mmap进行内存映射的时候需要加以判断，只有进程合法的地址区间才能够被映射。

#### ch8_05.rs

现象：运行后报错`unwrap()`一个None的对象，OS崩溃退出。

解释：与`ch8_01.rs`一致，都在进行大量的fork操作，会导致操作系统的内存资源迅速耗尽，进而`frame_alloc()`函数无法分配内存，返回类型`Option<>`在未加检查的情况下被直接`unwrap()`从而导致运行时错误。

解决办法：把所有`unwrap()`直接操作到的接口在`unwrap()`之前都判断内容非`None`，否则返回报错或者其他的错误信息。

#### ch8_07.rs

报错： range end index 29 out of range for slice of length 28

解释：试图建立大量文件，同时试图以非法字符串建立文件（这个没有改动程序便看不出来）。

解决办法：在`vfs`下设置文件同时可以存在的数量上限，创建的时候检查文件名的合法性以及数量的合法性，并且返回创建成功或者失败的信息，在系统调用中可以利用这些信息来避免崩溃。

### 实现内容

解决了`ch8_01.rs, ch8_05.rs, ch8_02.rs`中的问题。

### ch8_01.rs ch8_05.rs的解决

在所有的`frame_alloc()`返回结果直接`unwrap()`的地方都修改成

```rust
fn func_name() -> Option<xxx> {
  if let Some(xxx) = frame_alloc() {
     // do something
    Some(xxx)
  } else {
    None
  }
}
```

的模式，这里涉及到的函数主要有

```
MemorySet::map_one, 
PageTable::map,
MemorySet::find_pte_create,
insert_framed_area
MemorySet::new_bare,
MemorySet::new,
PageTable::push,
MemorySet::from_existed_user
```

等等。原先函数如果有返回值，但没有加`Option`则在返回值外面套一层`Option`；如果原先没有返回值，则返回一个`bool`来指示该操作是否成功；如果有`Option`返回值则在操作不成功的时候返回`None`

### ch8_02.rs的解决

在解决上一个问题的基础上，顺带便对`mmap`涉及到的所有内存操作都进行安全性检查，范式与上述的一致，具体涉及到的函数有

```
MemorySet::insert_framed_area,
MemorySet::push,
MemorySet::from_existed_user
```

更新以后，就不会对一块`None`的`unwrap()`，因而就能够安全地返回结果了。

### 扩展内容

我做了14，为shell加上`pipe`功能。

这个功能是可以避免对`os`进行改动的，只修改`user/`中的`usershell.rs`来实现。

具体而言，可以这样分析：`|`切分所有命令都有一个输入端`input`和一个输出端`output`。我们要做的是根据`|`和`>,<`的重定向情况，对文件添加重定向。

如果一个命令要用到上一个命令的输出流，需要满足

```
1. 上一个命令输出没有重定向到文件中去
2. 这一个命令输入没有被其他文件重定向
```

那么就可以用上一个命令的标准输出（STDOUT）作为输入了

同理，如果一个命令的输出要作为下一个命令输入，而不是STDOUT显示出来，则需要满足

```
1. 这个命令不是最后一个命令
2. 这个命令没有重定向到其他的文件中去
```

可以看出，可以维护两个OS文件`ftmp1, ftmp2`来代理这个过程。具体而言即为：

```
1. 在处理命令之前，先判断input是否需要被额外重定向（用上一阶段的ftmp）
2. 在处理命令之前，判断output是否需要被额外重定向（为下一阶段预置ftmp）
```

那么，在代码中，只需要添加这样一段逻辑即可完成该操作

```rust
for (cidx, cmd_line) in pipes.iter().enumerate() {
  // do something
  // input and output is redirected for the first time
  if cidx % 2 == 0 { // 1 as in, 2 as out
      if input.is_empty() && ftmp1 == true {
        input = nametmp1.clone();
      }
      if output.is_empty() && cidx != (pipes.len() - 1) {
        output = nametmp2.clone();
        ftmp2 = true; // next time this file is prepared
      } else {
        ftmp2 = false; // next time this file is not prepared
      }
    } else if cidx % 2 == 1 { // 2 is in, 1 is out
      if input.is_empty() && ftmp2 == true {
        input = nametmp2.clone();
      } 
      if output.is_empty() && cidx != (pipes.len() - 1) { // to stdout
        output = nametmp1.clone();
        ftmp1 = true;
      } else {
        ftmp1 = false;
      }
    }
  // exec the command as usual
}
```

在测试的时候，我设计了`test.rs`的文件，该文件接收从STDIN流中的字符，并且打印到`STDOUT`中去（by default）。

可以进行如下的测试

```shell
cmdline_args 1 2 3 4 | test | test | test | test | test
```

输出结果：

```shell
Shell: Process 2 exited with code 0
Shell: Process 2 exited with code 0
Shell: Process 2 exited with code 0
Shell: Process 2 exited with code 0
Shell: Process 2 exited with code 0
argc = 5
argv[0] = cmdline_args
argv[1] = 1
argv[2] = 2
argv[3] = 3
argv[4] = 4
Shell: Process 2 exited with code 0
```

可以发现`cmdline_args`的输出被“接力”着到最后一个test输出，同时还可以

```shell
test < ftmp2
```

发现`ftmp2`保存了`cmdline_arg2`的输出内容。

```shell
argc = 5
argv[0] = cmdline_args
argv[1] = 1
argv[2] = 2
argv[3] = 3
argv[4] = 4
Shell: Process 2 exited with code 0
```

`test.rs`的内容如下

```rust
extern crate user_lib;
use user_lib::{
    ch8::{forktest, hash},
    mmap,
    console::{getchar},
    write,
    read,
    STDIN,
    STDOUT,
};

#[no_mangle]
pub unsafe fn main(argc: usize, argv: &[&str]) -> i32 {
    let mut buf = [0u8; 1];
    loop {
        let size = read(STDIN, &mut buf) as usize;
        if size == 0 {
            break;
        }
        write(STDOUT, &buf[0..size]);
    }
    0
}
```



