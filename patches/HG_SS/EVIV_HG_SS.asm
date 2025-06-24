; EVIV_HG_SS.asm

.nds
.thumb

; ------- Inject hook into arm9.bin -------
.open "arm9.bin", 0x02000000

.org 0x020899C4

    bl EV_IV_Viewer

.close

; ------- Write function to synthOverlay 0000 -------
.open "unpacked/synthOverlay/0000", 0x023C8000


INJECT_ADDR equ 0x023C8000
.org INJECT_ADDR
.ascii "EV+IV Viewer"

EV_IV_Viewer:
    push {lr}
    bl 0x0206FD00           ; original function
    push {r0}

    sub sp, #0x4            ; allocate space for X on stack
    mov r0, #0x0            ; put 0 into r0 to send to stack
    str r0, [sp, #0x0]      ; Put it at start

    ldr  r3, =0x021D110C    ; gSystem
    ldr  r2, [r3, #0x44]    ; heldKeys
    ldr  r3, =0x0100        ; R button
    tst  r2, r3
    bne  .write_iv_ids

    ldr  r3, =0x021D110C
    ldr  r2, [r3, #0x44]
    ldr  r3, =0x0200        ; L button
    tst  r2, r3
    beq  .write_stats_ids

.write_ev_ids:
    mov r1, #0x7A
    lsl r1, r1, #0x4
    add r1, #0xC
    ldr r0, [r7, r1]    ; Pointer in sumamry struct to string variable for some reason
    bl 0x020263AC           ; String_SetEmpty
    ldr r1, =evString      ; "EV" + EOS
    bl 0x020269A0           ; CopyU16ArrayToString
    mov r0, r7              ; Summary Struct param 1
    mov r1, #0x2            ; Window ID 2 is param 2
    ldr r2, =0xe0f00        ; Color white is param 3
    bl 0x0208C850           ; sub_0208C850(int sumamryStruct,int windowID,s32 msgBank,u32 colour,u32 x)

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
    b .return

.write_iv_ids:
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
    b .return

.write_stats_ids:
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

.return:
    add sp, #0x4
    pop {r0,pc}

evString:
    .byte 0x00, 0xB0, 0x00, 0xC1, 0xFF, 0xFF    ; "EV" + EOS

.pool


.close