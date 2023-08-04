.globl context_stack
context_stack:
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

    stp q30, q31, [sp, -32]!
    stp q28, q29, [sp, -32]!
    stp q26, q27, [sp, -32]!
    stp q24, q25, [sp, -32]!
    stp q22, q23, [sp, -32]!
    stp q20, q21, [sp, -32]!
    stp q18, q19, [sp, -32]!
    stp q16, q17, [sp, -32]!

    stp q14, q15, [sp, -32]!
    stp q12, q13, [sp, -32]!
    stp q10, q11, [sp, -32]!
    stp q8, q9, [sp, -32]!
    stp q6, q7, [sp, -32]!
    stp q4, q5, [sp, -32]!
    stp q2, q3, [sp, -32]!
    stp q0, q1, [sp, -32]!

    mrs x8, TPIDR_EL1
    mrs x9, TTBR0_EL1
    stp x8, x9, [sp, -16]!

    mrs x8, SPSR_EL1
    mrs x9, ELR_EL1
    stp x8, x9, [sp, -16]!
    ret

.globl context_restore
context_restore:
    ldp x0, x1, [sp], 16
    msr SPSR_EL1, x0
    msr ELR_EL1, x1

    ldp x0, x1, [sp], 16
    msr TPIDR_EL1, x0
    msr TTBR0_EL1, x1

    ldp q0, q1, [sp], 32
    ldp q2, q3, [sp], 32
    ldp q4, q5, [sp], 32
    ldp q6, q7, [sp], 32
    ldp q8, q9, [sp], 32
    ldp q10, q11, [sp], 32
    ldp q12, q13, [sp], 32
    ldp q14, q15, [sp], 32

    ldp q16, q17, [sp], 32
    ldp q18, q19, [sp], 32
    ldp q20, q21, [sp], 32
    ldp q22, q23, [sp], 32
    ldp q24, q25, [sp], 32
    ldp q26, q27, [sp], 32
    ldp q28, q29, [sp], 32
    ldp q30, q31, [sp], 32

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
