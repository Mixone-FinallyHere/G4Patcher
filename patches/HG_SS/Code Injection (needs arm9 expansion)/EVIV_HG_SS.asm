; EVIV_HG_SS.asm

.nds
.thumb

AButton equ 1
BButton equ 2
SelectButton equ 4
StartButton equ 8
DPadRight equ 16
DPadLeft equ 32 ; i am not sure about the exact orientations of the DPad
DPadUp equ 64
DPadDown equ 128
RButton equ 256
LButton equ 512
XButton equ 1024

; ------- Inject hook into arm9.bin -------
.open "arm9.bin", 0x02000000

.org 0x02088b74

    bl EV_IV_Viewer

.close

; ------- Write function to synthOverlay 0000 -------
.open "unpacked/synthOverlay/0000", 0x023C8000


INJECT_ADDR equ 0x023C8000
.org INJECT_ADDR
.ascii "EV+IV_Viewer_start"

EV_IV_Viewer:
    ldr     r1, =RButton
    tst     r1, r4
    bne     check_current_mode     ; if R is clicked, branch to check_current_mode
    b       normal_return    ; if neither button is clicked, return to the original function

check_current_mode:
    push    {r0} ; Save r0 for later use
    ; Load current byte at 0x0208d3b6 to determine mode
    ldr     r0, =0x020899CC
    ldrb    r1, [r0]

    ; Check if it's IV mode, write EV IDs
    cmp     r1, #70
    beq     write_ev_ids

    ; Check if it's Stats mode, write IV IDs
    cmp     r1, #0xA3
    beq     write_iv_ids

    ; Default: start with Stats mode
    b       write_stats_ids

write_ev_ids:
    ldr r0, =0x020899CC
    mov r1, #13
    strb r1, [r0]
    ldr r0, =0x020899D8
    mov r1, #13
    strb r1, [r0]
    ldr r0, =0x020899E4
    mov r1, #14
    strb r1, [r0]
    ldr r0, =0x020899F0
    mov r1, #15
    strb r1, [r0]
    ldr r0, =0x020899FC
    mov r1, #17
    strb r1, [r0]
    ldr r0, =0x02089A08
    mov r1, #18
    strb r1, [r0]
    ldr r0, =0x02089A14
    mov r1, #16
    strb r1, [r0]
    b       refresh_screen

write_iv_ids:
    ldr r0, =0x020899CC
    mov r1, #70
    strb r1, [r0]
    ldr r0, =0x020899D8
    mov r1, #70
    strb r1, [r0]
    ldr r0, =0x020899E4
    mov r1, #71
    strb r1, [r0]
    ldr r0, =0x020899F0
    mov r1, #72
    strb r1, [r0]
    ldr r0, =0x020899FC
    mov r1, #74
    strb r1, [r0]
    ldr r0, =0x02089A08
    mov r1, #75
    strb r1, [r0]
    ldr r0, =0x02089A14
    mov r1, #73
    strb r1, [r0]
    b       refresh_screen

write_stats_ids:
    ldr r0, =0x020899CC
    mov r1, #0xA3
    strb r1, [r0]
    ldr r0, =0x020899D8
    mov r1, #0xA4
    strb r1, [r0]
    ldr r0, =0x020899E4
    mov r1, #0xA5
    strb r1, [r0]
    ldr r0, =0x020899F0
    mov r1, #0xA6
    strb r1, [r0]
    ldr r0, =0x020899FC
    mov r1, #0xA8
    strb r1, [r0]
    ldr r0, =0x02089A08
    mov r1, #0xA9
    strb r1, [r0]
    ldr r0, =0x02089A14
    mov r1, #0xA7
    strb r1, [r0]

refresh_screen:
    pop     {r0}
    mov     r1,  #0x0
    bl      0x0208a2c0 ; ChangeSummaryMon(summaryScreen, 0) ; refreshes the summary screen
	mov     r0, #0x13
    pop     {r4, r5, r6, pc}

normal_return:
    mov     r1,#0x10
    tst     r1, r4
    bx      lr

.pool

.ascii "EV+IV_Viewer_end"

.close