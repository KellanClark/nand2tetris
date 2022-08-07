// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Fill.asm

// Runs an infinite loop that listens to the keyboard input.
// When a key is pressed (any key), the program blackens the screen,
// i.e. writes "black" in every pixel;
// the screen should remain fully black as long as the key is pressed. 
// When no key is pressed, the program clears the screen, i.e. writes
// "white" in every pixel;
// the screen should remain fully clear as long as no key is pressed.

// while (1) {
//     int color = 0;
//     if (KBD > 0)
//         color -= 1;
//     int counter = 0x2000;
//     int *index = SCREEN;
//     do {
//         *index = color;
//         ++index;
//     } while (--counter > 0);
// }

(LOOP)
    @color
    M=0

    @KBD
    D=M
    @NEXT
    D;JEQ
    @color
    M=M-1

(NEXT)
    @8192
    D=A
    @counter
    M=D

    @SCREEN
    D=A
    @index
    M=D

(FILL)
    @color
    D=M
    @index
    A=M
    M=D

    D=A+1
    @index
    M=D

    @counter
    MD=M-1
    @FILL
    D;JGT

    @LOOP
    0;JMP
