# OS chapter5 实验报告

***2017011998 冯卓尔 计86***

---

## 编程内容

- 移植代码
- 在原先fork和exec的基础上，将exec的task通过fork函数的方式进行实现，具体而言就是fork出的进程作为new_task，exec请求的资源作为data，然后用new_task执行exec访问data



