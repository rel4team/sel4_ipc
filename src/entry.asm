.section .text.entry
    .global _start
    .global trap_entry
_start:
    mrs x19, mpidr_el1         // 读取 MPIDR_EL1 寄存器的值到 x19
    and x19, x19, #0xff         // 从 x19 中提取 CPU ID
    cbz x19, 1f                 // 如果 CPU ID 为 0，则跳转到 1f
    b .                         // 如果 CPU ID 不为 0，则无条件跳转到当前位置
1:
    adrp x8, boot_stack_top     // 将 boot_stack_top 的地址的高 12 位加载到 x8
    add x8, x8, 4096 * 16      // 将 boot_stack_top 的地址加上 4096 * 16，即栈的大小，存储到 x8

    mov sp, x8                 // 初始化栈指针 sp
    bl switch_to_el1      // 调用函数 switch_to_el1

    mov x0, x19               // 将 CPU ID 存储到 x0
    ldr x8, =call_test_main    // 将 call_test_main 的地址加载到 x8
    blr x8                    // 调用 call_test_main

    .section .bss.stack
    .global boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16          // 为栈分配空间
    .global boot_stack_top
boot_stack_top: