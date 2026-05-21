/* 00444094 __msize */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __msize
   
   Library: Visual Studio 2003 Release */

size_t __cdecl __msize(void *_Memory)

{
  uint uVar1;
  SIZE_T SVar2;
  size_t local_24;
  
  if (DAT_0046cd70 == 3) {
    __lock(4);
    uVar1 = ___sbh_find_block((int)_Memory);
    if (uVar1 != 0) {
      local_24 = *(int *)((int)_Memory + -4) - 9;
    }
    FUN_00444101();
    if (uVar1 != 0) {
      return local_24;
    }
  }
  SVar2 = HeapSize(DAT_0046cd6c,0,_Memory);
  return SVar2;
}
