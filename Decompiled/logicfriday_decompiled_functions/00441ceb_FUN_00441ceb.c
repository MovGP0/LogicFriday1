/* 00441ceb FUN_00441ceb */

undefined4 __cdecl FUN_00441ceb(int param_1)

{
  void *_Memory;
  int *piVar1;
  undefined4 uVar2;
  int iVar3;
  
  piVar1 = &DAT_00451e40 + param_1 * 2;
  if (*piVar1 != 0) {
    return 1;
  }
  _Memory = _malloc(0x18);
  if (_Memory == (void *)0x0) {
LAB_00441d12:
    piVar1 = FUN_00441a24();
    *piVar1 = 0xc;
    uVar2 = 0;
  }
  else {
    __lock(10);
    if (*piVar1 == 0) {
      iVar3 = ___crtInitCritSecAndSpinCount(_Memory,4000);
      if (iVar3 == 0) {
        _free(_Memory);
        FUN_00441cd6(10);
        goto LAB_00441d12;
      }
      *piVar1 = (int)_Memory;
    }
    else {
      _free(_Memory);
    }
    FUN_00441cd6(10);
    uVar2 = 1;
  }
  return uVar2;
}
