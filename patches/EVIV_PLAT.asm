; EVIV_PLAT.asm

.nds
.thumb

; ------- Inject hook into arm9.bin -------
.open "arm9.bin", 0x02000000

.org 0x0208D602

    bl EV_IV_Viewer

.close


; ------- Write function to overlay_0000 -------
.open "unpacked/synthOverlay/0009", 0x023C8000


INJECT_ADDR equ 0x023C8750
.org INJECT_ADDR

EV_IV_Viewer:
    ; check if R is held
    ldr  r3, =0x021BF67C     ; gSystem
    ldr  r2, [r3, #0x44]     ; gSystem->heldKeys
    ldr r3, =256             ; R button
    tst  r2, r3
    bne .ivs              ; if R is held, branch to IVs section
    ldr  r3, =0x021BF67C     ; gSystem
    ldr  r2, [r3, #0x44]     ; gSystem->heldKeys
    ldr r3, =512             ; L button
    tst  r2, r3
    beq  .normal_return      ; skip if not held

.evs:

    ; r5 = monData, r6 = mon
    mov  r4, r5              ; r4 = monData
    mov  r7, r6              ; r7 = mon

    ; --- HP IV (currentHP slot)
    mov r0, r7              ; mon
    mov r1, #13           ; MON_DATA_HP_IV
    mov r2, #0
    bl   0x02074470          ; Pokemon_GetValue
    strh  r0, [r4, #0x24]

    ; --- HP IV (maxHP slot)
    mov r0, r7              ; mon
    mov r1, #13           ; MON_DATA_HP_IV
    mov r2, #0
    bl   0x02074470          ; Pokemon_GetValue
    strh  r0, [r4, #0x26]

    ; --- Atk IV
    mov r0, r7
    mov r1, #14
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x28]

    ; --- Def IV
    mov r0, r7
    mov r1, #15
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2A]

    ; --- SpAtk IV
    mov r0, r7
    mov r1, #17
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2C]

    ; --- SpDef IV
    mov r0, r7
    mov r1, #18
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2E]

    ; --- Speed IV
    mov r0, r7
    mov r1, #16
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x30]

    b  .normal_return      ; skip if not held

.ivs:

    ; r5 = monData, r6 = mon
    mov  r4, r5              ; r4 = monData
    mov  r7, r6              ; r7 = mon

    ; --- HP IV (currentHP slot)
    mov r0, r7              ; mon
    mov r1, #70           ; MON_DATA_HP_IV
    mov r2, #0
    bl   0x02074470          ; Pokemon_GetValue
    strh  r0, [r4, #0x24]

    ; --- HP IV (maxHP slot)
    mov r0, r7              ; mon
    mov r1, #70           ; MON_DATA_HP_IV
    mov r2, #0
    bl   0x02074470          ; Pokemon_GetValue
    strh  r0, [r4, #0x26]

    ; --- Atk IV
    mov r0, r7
    mov r1, #71
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x28]

    ; --- Def IV
    mov r0, r7
    mov r1, #72
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2A]

    ; --- SpAtk IV
    mov r0, r7
    mov r1, #74
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2C]

    ; --- SpDef IV
    mov r0, r7
    mov r1, #75
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x2E]

    ; --- Speed IV
    mov r0, r7
    mov r1, #73
    mov r2, #0
    bl   0x02074470
    strh r0, [r4, #0x30]

.normal_return:
    ; replicate tail of original function:
    ldr  r1, [sp, #4]        ; reencrypt flag
    mov r0, r6              ; r0 = mon
    bl   0x02073CD4          ; Pokemon_ExitDecryptionContext(mon, reencrypt)
    add  sp, #8
    pop  {r3, r4, r5, r6, r7, pc}

.pool

.close