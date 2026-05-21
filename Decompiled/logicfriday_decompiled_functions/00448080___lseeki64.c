/* 00448080 __lseeki64 */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __lseeki64
   
   Library: Visual Studio 2003 Release */

longlong __cdecl __lseeki64(int _FileHandle,longlong _Offset,int _Origin)

{
  int *piVar1;
  ulong *puVar2;
  int iVar3;
  LONG in_stack_00000008;
  undefined8 local_24;
  
  if ((uint)_FileHandle < DAT_0046cc2c) {
    iVar3 = (_FileHandle & 0x1fU) * 0x24;
    if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) != 0) {
      FUN_00446154(_FileHandle);
      if ((*(byte *)((&DAT_0046cc40)[_FileHandle >> 5] + 4 + iVar3) & 1) == 0) {
        piVar1 = FUN_00441a24();
        *piVar1 = 9;
        puVar2 = FUN_00441a2d();
        *puVar2 = 0;
        local_24 = 0xffffffffffffffff;
      }
      else {
        local_24 = __lseeki64_lk(_FileHandle,in_stack_00000008,(LONG)_Offset,_Offset._4_4_);
      }
      FUN_00448118();
      goto LAB_00448138;
    }
  }
  piVar1 = FUN_00441a24();
  *piVar1 = 9;
  puVar2 = FUN_00441a2d();
  *puVar2 = 0;
  local_24._4_4_ = 0xffffffff;
  local_24._0_4_ = 0xffffffff;
LAB_00448138:
  return CONCAT44(local_24._4_4_,(undefined4)local_24);
}
