// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/04/Mult.asm

// Multiplies R0 and R1 and stores the result in R2.
// (R0, R1, R2 refer to RAM[0], RAM[1], and RAM[2], respectively.)
//
// This program only needs to handle arguments that satisfy
// R0 >= 0, R1 >= 0, and R0*R1 < 32768.

// int counter = 16;
// int mask = 0x0001;
// R2 = 0;
// do {
//     if (R0 & mask) {
//         R2 += R1;
//     }
//     mask <<= 1;
//     R1 <<= 1;
// } while (--counter > 0);

// Put your code here.
    @16
    D=A
    @counter
    M=D
    @mask
    M=1
    @R2
    M=0

(LOOP)
    @R0
    D=M
    @mask
    D=D&M
    @NEXT
    D;JEQ

    @R1
    D=M
    @R2
    M=D+M

(NEXT)
    @mask
    D=M
    M=D+M

    @R1
    D=M
    M=D+M

    @counter
    MD=M-1
    @LOOP
    D;JGT

(END)
    0;JMP
