/* 00446336 __commit */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __commit
   
   Library: Visual Studio 2003 Release */

int __cdecl __commit(int _FileHandle)

{
  HANDLE hFile;
  BOOL BVar1;
  ulong *puVar2;
  int *piVar3;
  int iVar4;
  DWORD local_20;
  
  if (DAT_0046cc2c <= (uint)_FileHandle) {
LAB_004463de:
    piVar3 = FUN_00441a24();
    *piVar3 = 9;
    return -1;
  }
  iVar4 = (_FileHandle & 0x1fU) * 0x24;
  if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar4) & 1) == 0) goto LAB_004463de;
  FUN_00446154(_FileHandle);
  if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar4) & 1) != 0) {
    hFile = (HANDLE)__get_osfhandle(_FileHandle);
    BVar1 = FlushFileBuffers(hFile);
    if (BVar1 == 0) {
      local_20 = GetLastError();
    }
    else {
      local_20 = 0;
    }
    if (local_20 == 0) goto LAB_004463c5;
    puVar2 = FUN_00441a2d();
    *puVar2 = local_20;
  }
  piVar3 = FUN_00441a24();
  *piVar3 = 9;
  local_20 = 0xffffffff;
LAB_004463c5:
  FUN_004463d6();
  return local_20;
}
