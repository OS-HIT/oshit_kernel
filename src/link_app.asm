
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad 2
    .quad app_0_start
    .quad app_1_start
    .quad app_1_end

    .section .data
    .global app_0_start
    .global app_0_end
    .align 3
app_0_start:
    .incbin "../user_bins/usr_hello_world_a"
app_0_end:

    .section .data
    .global app_1_start
    .global app_1_end
    .align 3
app_1_start:
    .incbin "../user_bins/usr_hello_world_b"
app_1_end:
# Try to make cargo happy: last compiled @ Wed, 21 Apr 2021 09:25:08 +0000
