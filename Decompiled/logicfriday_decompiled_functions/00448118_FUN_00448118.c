/* 00448118 FUN_00448118 */

void FUN_00448118(void)

{
  int unaff_EBX;
  
  __unlock_fhandle(unaff_EBX);
  return;
}



/* 0044813e FID_conflict:__getenv_lk */

/* Library Function - Multiple Matches With Different Base Names
    __getenv_lk
    _getenv
   
   Library: Visual Studio 2003 Release */

char * __cdecl FID_conflict___getenv_lk(char *_VarName)

{
  int iVar1;
  size_t _MaxCount;
  size_t sVar2;
  int *piVar3;
  
  if (DAT_0046cd40 == 0) {
    return (char *)0x0;
  }
  if (((DAT_0046c700 != (int *)0x0) ||
      (((DAT_0046c708 != 0 && (iVar1 = FUN_004490a6(), iVar1 == 0)) && (DAT_0046c700 != (int *)0x0))
      )) && (piVar3 = DAT_0046c700, _VarName != (char *)0x0)) {
    _MaxCount = _strlen(_VarName);
    for (; (char *)*piVar3 != (char *)0x0; piVar3 = piVar3 + 1) {
      sVar2 = _strlen((char *)*piVar3);
      if (((_MaxCount < sVar2) && (((uchar *)*piVar3)[_MaxCount] == '=')) &&
         (iVar1 = __mbsnbicoll((uchar *)*piVar3,(uchar *)_VarName,_MaxCount), iVar1 == 0)) {
        return (char *)(*piVar3 + 1 + _MaxCount);
      }
    }
  }
  return (char *)0x0;
}
