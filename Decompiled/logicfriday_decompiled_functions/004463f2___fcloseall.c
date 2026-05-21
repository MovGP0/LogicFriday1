/* 004463f2 __fcloseall */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Single Match
    __fcloseall
   
   Library: Visual Studio 2003 Release */

int __cdecl __fcloseall(void)

{
  FILE *_File;
  int iVar1;
  int iVar2;
  int iVar3;
  int local_20;
  
  local_20 = 0;
  __lock(1);
  for (iVar3 = 3; iVar3 < DAT_0046dd80; iVar3 = iVar3 + 1) {
    iVar2 = iVar3 * 4;
    _File = *(FILE **)(iVar2 + DAT_0046cd74);
    if (_File != (FILE *)0x0) {
      if ((_File->_flag & 0x83) != 0) {
        iVar1 = _fclose(_File);
        if (iVar1 != -1) {
          local_20 = local_20 + 1;
        }
      }
      if (0x13 < iVar3) {
        DeleteCriticalSection((LPCRITICAL_SECTION)(*(int *)(iVar2 + DAT_0046cd74) + 0x20));
        _free(*(void **)(iVar2 + DAT_0046cd74));
        *(undefined4 *)(iVar2 + DAT_0046cd74) = 0;
      }
    }
  }
  FUN_00446484();
  return local_20;
}
