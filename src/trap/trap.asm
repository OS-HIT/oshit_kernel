.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
.macro MOVE_USIZE n
    ld t1, \n*8(sp)
    sd t1, \n*8(t0)
.endm
    .section .text.trampoline
    .globl __alltraps
    .globl __restore
    .globl __restore_to_signal_handler
    .globl __siginfo
    .align 2
__alltraps:
    csrrw sp, sscratch, sp
    # now sp->*TrapContext in user space, sscratch->user stack
    # save other general purpose registers
    sd x1, 1*8(sp)
    # skip sp(x2), we will save it later
    sd x3, 3*8(sp)
    # skip tp(x4), application does not use it
    # save x5~x31
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # we can use t0/t1/t2 freely, because they have been saved in TrapContext
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # read user stack from sscratch and save it in TrapContext
    csrr t2, sscratch
    sd t2, 2*8(sp)
    # load kernel_satp into t0
    ld t0, 34*8(sp)
    # load trap_handler into t1
    ld t1, 36*8(sp)
    # move to kernel_sp
    ld sp, 35*8(sp)
    # switch to kernel space
    csrw satp, t0
    sfence.vma
    # jump to trap_handler
    jr t1

__restore:
    # a0: *TrapContext in user space(Constant); a1: user space token
    # switch to user space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp points to TrapContext in user space, start restoring based on it
    # restore sstatus/sepc
    ld t0, 32*8(sp)
    ld t1, 33*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    # restore general purpose registers except x0/sp/tp
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # back to user stack
    ld sp, 2*8(sp)
    sret


# pub fn __restore_to_signal_handler(trap_context: usize, user_satp: usize, handler_va: usize, signal: usize)
# handler_va -> pc
# original pc (epc) -> x1
# stack -> stack + 32
#   (supervisor mode)__restore_to_signal_handler
#       switch to user layout
#       move saved register value from trap_context to user_stack (growing stack down simultaneously)
#       set sepc to handler_va(signal_handler)
#       set return address(x1) to __user_restore_from_handler, thus signal_handler will "return" to it
#       set argument0 (a0) to a3(signal)
#       sret
#   (user mode)signal_handler
#       save callee save registers (we don't care)
#       do it's job
#       restore callee save registers
#       ret (jalr x0, x1, 0)    //? really?
#   (user mode)__user_restore_from_handler
#       prepare epc on a0
#       prepare syscall num on a7
#       ecall sys_sigreturn
#   (supervisor mode)__all_traps
#   (supervisor mode)trap_handler
#   (supervisor mode)sys_sigreturn
#       read user register value from user_stack (saved in __restore_to_signal_handler)
#       populate trap_context
#   (supervisor mode)__restore
#   (user mode)original execution sequence
__restore_to_signal_handler:
    # a0: *TrapContext in user space(Constant); a1: user space token; a2: handler_va; a3: &siginfo_t; 
    # switch to user space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    # make sp points to TrapContext in user space
    mv sp, a0
    # make t0 points to sp in user space
    ld t0, 2*8(sp)
    # move everything from TrapContext to user stack.
    .set n, 0
    .rept 36
        MOVE_USIZE %n
        .set n, n+1
    .endr
    # save original epc
    ld t1, 33*8(sp)
    sd t1, 33*8(t0)
    
    # grow stack downward
    addi sp, sp, -36*8

    # reset sstatus
    ld t0, 32*8(sp)
    csrw sstatus, t0
    
    # set sepc to a2: handler_va
    csrw sepc, a2

    # set return address 2 __user_restore_from_handler
    la t2, __user_restore_from_handler
    la t3, __alltraps
    sub x1, t2, t3
    la t2, TRAMPOLINE
    add x1, t2, x1
    ld x1, 0(x1)
    # set argument 0 to signal, 1 to siginfo_t, 2 to none (not implementing ucontext_t)
    mv a0, a3
    mv a1, a4
    li a2, 0

    # sret to user mode @ handler_va(a2)
    sret

# run in user mode
__user_restore_from_handler:
    # restore stack pointer. This now should point directly at head of a TrapContext, the one saved by __restore_to_signal_handler
    addi sp, sp, 36*8
    # syscall num for sys_sigreturn: 
    li a7, 139
    ecall


.align 4
__siginfo:
    .space 64