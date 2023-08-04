.macro HANDLER type, desc
    .balign 0x80
    stp lr, xzr, [sp, -16]! // Save lr before bl
    bl context_stack

    mov x0, \type
    movk x0, \desc, lsl 16
    mov x1, sp
    bl exception_handler

    bl context_restore
    ldp lr, xzr, [sp], 16 // Restore lr after bl
    eret
.endm

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
