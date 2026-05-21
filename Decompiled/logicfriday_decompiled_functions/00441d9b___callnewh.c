/* 00441d9b __callnewh */

/* Library Function - Single Match
    __callnewh
   
   Library: Visual Studio 2003 Release */

int __cdecl __callnewh(size_t _Size)

{
  int iVar1;
  
  if (DAT_0046c6c8 != (code *)0x0) {
    iVar1 = (*DAT_0046c6c8)(_Size);
    if (iVar1 != 0) {
      return 1;
    }
  }
  return 0;
}
