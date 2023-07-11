<p align="center">
    <h2 align="center">Y86-64 Interpreter</h2>
</p>

A terribly structured and not-at-all-idiomatic implementation of the y86 assembly language described in [this textbook](https://csapp.cs.cmu.edu/).  

I originally did this as a class project in C.

## Usage
y86 'object' files are expected as input, which take the following form:
```
<ADDRESS>: <BYTE ENCODING> | <READABLE ASM>
```
For example, the following y86 asm...  
```
nop
rrmovq %rax, %rbx
xorq %rax, %rax
halt
```
would produce the following object file:  
```
0x0000: 10                     | nop
0x0001: 2003                   | rrmovq %rax, %rbx
0x0003: 6300                   | xorq %rax, %rax
0x0005: 00                     | halt
```

You can also use an [online simulator](https://boginw.github.io/js-y86-64/) to produce object code, though leaving breakpoints in the object code will result in errors.  

