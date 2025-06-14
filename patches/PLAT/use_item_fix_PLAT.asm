.nds
.thumb

.open "overlay/overlay_0014.bin", 0x0221FC20  ; Open the trainer AI overlay

.org 0x0222487a
    
    bl function

.close

.open "unpacked/synthOverlay/0009", 0x023C8000  ; Open the synth overlay

INJECT_ADDR equ 0x023C8000
.org INJECT_ADDR
.ascii "use_item_fix"
.align 2
function:
    push {lr}
    sub r0, #0x3a
    strh r2, [r1, r0]
    ldr  r0, [sp, #8] 
    add r0, r0, #4 ; Set counter variable to >= 4, this acts as a break condition
    str  r0, [sp, #8]
    pop {pc}
.close