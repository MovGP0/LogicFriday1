/* 0043e630 FUN_0043e630 */

void FUN_0043e630(void)

{
  FILE *unaff_ESI;
  
  __unlock_file(unaff_ESI);
  return;
}



/* 0043e638 FID_conflict:_fwprintf */

/* WARNING: Function: __SEH_prolog replaced with injection: SEH_prolog */
/* WARNING: Function: __SEH_epilog replaced with injection: EH_epilog3 */
/* Library Function - Multiple Matches With Different Base Names
    _fprintf
    _fwprintf
   
   Library: Visual Studio 2003 Release */

int __cdecl FID_conflict__fwprintf(FILE *_File,wchar_t *_Format,...)

{
  int _Flag;
  int iVar1;
  
  __lock_file(_File);
  _Flag = __stbuf(_File);
  iVar1 = FUN_00441127(_File,(byte *)_Format,(wchar_t *)&stack0x0000000c);
  __ftbuf(_Flag,_File);
  FUN_0043e68c();
  return iVar1;
}
