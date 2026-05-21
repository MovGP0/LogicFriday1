/* 0044465e __read */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __read
   
   Library: Visual Studio 2003 Release */

int __cdecl __read(int _FileHandle,void *_DstBuf,uint _MaxCharCount)

{
  int *piVar1;
  ulong *puVar2;
  int iVar3;
  int local_20;
  
  if ((uint)_FileHandle < DAT_0046cc2c) {
    iVar3 = (_FileHandle & 0x1fU) * 0x24;
    if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) != 0) {
      FUN_00446154(_FileHandle);
      if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) == 0) {
        piVar1 = FUN_00441a24();
        *piVar1 = 9;
        puVar2 = FUN_00441a2d();
        *puVar2 = 0;
        local_20 = -1;
      }
      else {
        local_20 = FUN_0044448f(_FileHandle,_DstBuf,(char *)_MaxCharCount);
      }
      FUN_004446e5();
      return local_20;
    }
  }
  piVar1 = FUN_00441a24();
  *piVar1 = 9;
  puVar2 = FUN_00441a2d();
  *puVar2 = 0;
  return -1;
}
