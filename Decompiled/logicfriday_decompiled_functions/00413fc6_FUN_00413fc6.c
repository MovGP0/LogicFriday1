/* 00413fc6 FUN_00413fc6 */

undefined4 __thiscall FUN_00413fc6(void *this,undefined4 param_1)

{
  FILE *_File;
  undefined4 uVar1;
  
  DeleteFileA((LPCSTR)((int)this + 0x784));
  _File = (FILE *)FUN_0043e6f2((char *)((int)this + 0x784),"wt");
  if (_File == (FILE *)0x0) {
    uVar1 = 0x2f000b;
  }
  else {
    FID_conflict__fwprintf(_File,L"猥",param_1);
    _fclose(_File);
    uVar1 = 0;
  }
  return uVar1;
}
