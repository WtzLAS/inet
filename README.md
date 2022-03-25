# inet

一个用Rust写的Interaction Net实现。

## inet-compiler

- [x] 语法解析
- [ ] 代码生成

## inet-core

- [x] 单线程原型
- [x] 多线程优化
- [x] 统计信息
- [x] 支持原地(In-place)替换规则 (通过clear旧Agent并复用空间来间接实现)
- [ ] Weak Reduction
- [x] 支持Agent带数据 (现在使用`Box<dyn Any + Send + Sync>`来做类型擦除，有优化空间)
- [ ] ~~Rule匹配使用MPHF~~ (没有带来预期性能收益)

## inet-example

一个基本的加法演示。