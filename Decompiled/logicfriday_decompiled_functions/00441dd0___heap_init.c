/* 00441dd0 __heap_init */

/* Library Function - Single Match
    __heap_init
   
   Library: Visual Studio 2003 Release */

int __cdecl __heap_init(void)

{
  int iVar1;
  int in_stack_00000004;
  
  DAT_0046cd6c = HeapCreate((uint)(in_stack_00000004 == 0),0x1000,0);
  if (DAT_0046cd6c == (HANDLE)0x0) {
    return 0;
  }
  DAT_0046cd70 = ___heap_select();
  if ((DAT_0046cd70 == 3) && (iVar1 = ___sbh_heap_init(0x3f8), iVar1 == 0)) {
    HeapDestroy(DAT_0046cd6c);
    return 0;
  }
  return 1;
}
