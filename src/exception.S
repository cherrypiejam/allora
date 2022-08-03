context_stack:
    // stack corruptible registers
    stp x28, x29, [sp, -16]!
    stp x26, x27, [sp, -16]!
    stp x24, x25, [sp, -16]!
    stp x22, x23, [sp, -16]!
    stp x20, x21, [sp, -16]!
    stp x18, x19, [sp, -16]!
    stp x16, x17, [sp, -16]!

    stp x14, x15, [sp, -16]!
    stp x12, x13, [sp, -16]!
    stp x10, x11, [sp, -16]!
    stp x8, x9, [sp, -16]!
    stp x6, x7, [sp, -16]!
    stp x4, x5, [sp, -16]!
    stp x2, x3, [sp, -16]!
    stp x0, x1, [sp, -16]!

    mrs x0, SPSR_EL1
    mrs x1, ELR_EL1
    stp x0, x1, [sp, -16]!
    ret

context_restore:
    // restore corruptible registers
    ldp x0, x1, [sp], 16
    msr SPSR_EL1, x0
    msr ELR_EL1, x1

    ldp x0, x1, [sp], 16
    ldp x2, x3, [sp], 16
    ldp x4, x5, [sp], 16
    ldp x6, x7, [sp], 16
    ldp x8, x9, [sp], 16
    ldp x10, x11, [sp], 16
    ldp x12, x13, [sp], 16
    ldp x14, x15, [sp], 16

    ldp x16, x17, [sp], 16
    ldp x18, x19, [sp], 16
    ldp x20, x21, [sp], 16
    ldp x22, x23, [sp], 16
    ldp x24, x25, [sp], 16
    ldp x26, x27, [sp], 16
    ldp x28, x29, [sp], 16
    ret

.macro HANDLER type, desc
    .balign 0x80

    // mov x0, 123
    // mov x28, 666

    stp lr, xzr, [sp, -16]! // save lr before bl
    bl context_stack

    mov x0, \type
    movk x0, \desc, lsl 16
    mov x1, sp
    bl exception_handler

    bl context_restore
    ldp lr, xzr, [sp], 16 // restore lr after bl

    eret
.endm

// go_system_off:
    // bl system_off
    // eret


.balign 0x800
.globl exception_vector_table
exception_vector_table:
    HANDLER 0, 0
    HANDLER 1, 0
    HANDLER 2, 0
    HANDLER 3, 0
    HANDLER 0, 1
    HANDLER 1, 1
    HANDLER 2, 1
    HANDLER 3, 1
    HANDLER 0, 2
    HANDLER 1, 2
    HANDLER 2, 2
    HANDLER 3, 2
    HANDLER 0, 3
    HANDLER 1, 3
    HANDLER 2, 3
    HANDLER 3, 3
