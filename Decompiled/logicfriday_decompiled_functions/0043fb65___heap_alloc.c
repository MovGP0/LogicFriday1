/* 0043fb65 __heap_alloc */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __heap_alloc
   
   Library: Visual Studio 2003 Release */

void * __cdecl __heap_alloc(size_t _Size)

{
  int *piVar1;
  LPVOID pvVar2;
  
  if ((DAT_0046cd70 == 3) && (_Size <= DAT_0046cd5c)) {
    __lock(4);
    piVar1 = ___sbh_alloc_block((uint *)_Size);
    FUN_0043fbd7();
    if (piVar1 != (int *)0x0) {
      return piVar1;
    }
  }
  if (_Size == 0) {
    _Size = 1;
  }
  if (DAT_0046cd70 != 1) {
    _Size = _Size + 0xf & 0xfffffff0;
  }
  pvVar2 = HeapAlloc(DAT_0046cd6c,0,_Size);
  return pvVar2;
}
