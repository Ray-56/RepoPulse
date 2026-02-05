# Notes

## 2026-02-04

规则：domain 不能依赖 application / infrastructure / interfaces。
application 可以依赖 domain。
infra 依赖 application（实现 ports）。
interfaces 调用 application（组装依赖）。