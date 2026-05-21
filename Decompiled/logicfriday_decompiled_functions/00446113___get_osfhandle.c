/* 00446113 __get_osfhandle */

/* Library Function - Single Match
    __get_osfhandle
   
   Library: Visual Studio 2003 Release */

intptr_t __cdecl __get_osfhandle(int _FileHandle)

{
  intptr_t *piVar1;
  int *piVar2;
  ulong *puVar3;
  
  if (((uint)_FileHandle < DAT_0046cc2c) &&
     (piVar1 = (intptr_t *)((&DAT_0046cc40)[_FileHandle >> 5] + (_FileHandle & 0x1fU) * 0x24),
     (*(byte *)(piVar1 + 1) & 1) != 0)) {
    return *piVar1;
  }
  piVar2 = FUN_00441a24();
  *piVar2 = 9;
  puVar3 = FUN_00441a2d();
  *puVar3 = 0;
  return -1;
}
