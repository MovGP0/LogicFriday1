/* 00414543 FUN_00414543 */

undefined4 __thiscall FUN_00414543(void *this,undefined4 param_1)

{
  FILE *_File;
  undefined4 uVar1;
  
  _File = (FILE *)FUN_0043e6f2((char *)((int)this + 0x98c),"w+t");
  if (_File == (FILE *)0x0) {
    uVar1 = 0x2b0001;
  }
  else {
    FID_conflict__fwprintf(_File,L"猥",param_1);
    _fclose(_File);
    uVar1 = 0;
  }
  return uVar1;
}
