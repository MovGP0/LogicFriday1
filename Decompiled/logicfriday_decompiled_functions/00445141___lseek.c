/* 00445141 __lseek */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __lseek
   
   Library: Visual Studio 2003 Release */

long __cdecl __lseek(int _FileHandle,long _Offset,int _Origin)

{
  int *piVar1;
  ulong *puVar2;
  int iVar3;
  DWORD local_20;
  
  if ((uint)_FileHandle < DAT_0046cc2c) {
    iVar3 = (_FileHandle & 0x1fU) * 0x24;
    if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) != 0) {
      FUN_00446154(_FileHandle);
      if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) == 0) {
        piVar1 = FUN_00441a24();
        *piVar1 = 9;
        puVar2 = FUN_00441a2d();
        *puVar2 = 0;
        local_20 = 0xffffffff;
      }
      else {
        local_20 = __lseek_lk(_FileHandle,_Offset,_Origin);
      }
      FUN_004451c8();
      return local_20;
    }
  }
  piVar1 = FUN_00441a24();
  *piVar1 = 9;
  puVar2 = FUN_00441a2d();
  *puVar2 = 0;
  return -1;
}
