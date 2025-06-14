; Prevent the player from using items in battle by Yako
; Credits: The pokeplatinum team for decompiling the game

.nds
.thumb

.open "overlay/overlay_0016.bin", 0x0223B140  ; Open the battle overlay

.org 0x0224be98
    
    bl no_items
    mov r0,r0

.close

.open "unpacked/synthOverlay/0009", 0x023C8000  ; Open the synth overlay

INJECT_ADDR equ 0x023C8170
.org INJECT_ADDR    ; Put function at defined offset
.ascii "no_items"
.align 2
no_items:
    push {lr}
    bl 0x0223DF0C    ; BattleSystem_BattleType
    mov r1,#0x01
    tst r4,r1
    bne ai
    mov r1,#0x01
    b end
    ai: 
    mov r1,#0x84
    end: pop {pc}

.close