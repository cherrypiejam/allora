.globl _start
.extern LD_STACK_PTR

.section ".text.boot"
_start:
    /* Get device tree. It's either passed in x0, or at the beginning of memory
     */
    adr x1, .
    mrs x2, TTBR0_EL1
    cmp x0,  #0
    mov x0, #0x40000000

    /* Enable NEON/SIMD instructions */
    mov x30, #(0x3 << 20)
    msr cpacr_el1, x30
    // msr cptr_el2, x30
    isb
    /* -- */

    /* Enable MMU */
    // ldr     x30, =LD_TTBR0_BASE
    // msr     TTBR0_EL2, x30
    // eor     x30, x30, x30
    // msr     TCR_EL2, x30
    // isb

    // mrs     x30, SCTLR_EL2
    // orr     x30, x30, 1
    // msr     SCTLR_EL2, x30
    // isb
    /* -- */

    /* Route exceptions to EL2 */
    mrs x3, HCR_EL2
    orr x3, x3, (1 << 27)
    msr HCR_EL2, x3
    isb

    ldr     x30, =LD_STACK_PTR0
    mov     sp, x30
    bl      kernel_main

.equ PSCI_SYSTEM_OFF, 0x84000008
.globl system_off
system_off:
    ldr     x0, =PSCI_SYSTEM_OFF
    smc     #0

start_core_1:
	/* Enable NEON/SIMD instructions */
	mov x30, #(0x3 << 20)
    msr cpacr_el1, x30
	// msr cptr_el2, x30
	isb

	ldr     x30, [x0, #8]
	add     x30, x30, #8192
	mov     sp, x30
	ldr     x3, [x0, #0]
	br      x3

spin:
	bl spin

.equ PSCI_0_2_FN64_CPU_ON, 0xc4000003
.globl cpu_on
cpu_on:
	mov x3, x1         /* context ID: put into target CPU's x0 */
	mov x1, x0         /* target cpu */
	ldr x0, =PSCI_0_2_FN64_CPU_ON
	ldr x2, =start_core_1   /* entry point */
	smc #0
		ret

.equ PSCI_CPU_OFF, 0x84000002
.globl cpu_off
cpu_off:
	ldr     x0, =PSCI_CPU_OFF
	smc     #0
		ret

spin_lock:
	mov	w2, #1
	sevl
l1:	wfe
l2:	ldaxr	w1, [x0]
	cbnz	w1, l1
	stxr	w1, w2, [x0]
	cbnz	w1, l2
	ret

