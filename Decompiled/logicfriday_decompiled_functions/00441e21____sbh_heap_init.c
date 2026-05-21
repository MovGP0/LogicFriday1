/* 00441e21 ___sbh_heap_init */

/* Library Function - Single Match
    ___sbh_heap_init
   
   Library: Visual Studio 2003 Release */

undefined4 __cdecl ___sbh_heap_init(undefined4 param_1)

{
  DAT_0046cd58 = HeapAlloc(DAT_0046cd6c,0,0x140);
  if (DAT_0046cd58 == (LPVOID)0x0) {
    return 0;
  }
  DAT_0046cd50 = 0;
  DAT_0046cd54 = 0;
  DAT_0046cd60 = DAT_0046cd58;
  DAT_0046cd5c = param_1;
  DAT_0046cd64 = 0x10;
  return 1;
}
