/* 0041a687 FUN_0041a687 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0041a687(void *this,int param_1)

{
  BOOL BVar1;
  undefined4 uVar2;
  int iVar3;
  ulonglong uVar4;
  uint unaff_retaddr;
  char local_a4 [132];
  uint local_20;
  int local_1c;
  int local_18;
  int local_14;
  DEVMODEA *local_10;
  int local_c;
  
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  if (*(int *)(param_1 + 0x1650) == 0) {
    *(undefined4 *)((int)this + 0xc0) = 1;
  }
  else {
    *(undefined4 *)((int)this + 0xc0) = 0;
  }
  if (*(int *)((int)this + 0xbc) == 0) {
    FUN_0041a5b2(this);
    *(undefined4 *)((int)this + 0xbc) = 1;
  }
  *(int *)((int)this + 0xc) = param_1;
  _memset((void *)((int)this + 0xc4),0,0x14);
  *(undefined4 *)((int)this + 0xc4) = 0x14;
  *(char **)((int)this + 200) = "Logic Friday: Printing";
  *(undefined4 *)((int)this + 0xd8) = 0x42;
  *(undefined4 *)((int)this + 0xdc) = *(undefined4 *)this;
  *(undefined4 *)((int)this + 0xe0) = *(undefined4 *)((int)this + 0x10);
  *(undefined4 *)((int)this + 0xe4) = *(undefined4 *)((int)this + 0x14);
  *(undefined4 *)((int)this + 0xe8) = 0;
  *(undefined4 *)((int)this + 0xec) = 0x145114;
  *(undefined2 *)((int)this + 0xf0) = 0;
  *(undefined2 *)((int)this + 0xf2) = 0;
  *(undefined2 *)((int)this + 0xf4) = 1;
  *(undefined2 *)((int)this + 0xf6) = 10;
  *(undefined2 *)((int)this + 0xf8) = 1;
  *(undefined4 *)((int)this + 0xfa) = *(undefined4 *)((int)this + 8);
  *(undefined4 *)((int)this + 0xfe) = 0;
  *(code **)((int)this + 0x102) = FUN_0040aefe;
  *(undefined4 *)((int)this + 0x106) = 0;
  *(char **)((int)this + 0x10a) = "PRINTDLG";
  *(undefined4 *)((int)this + 0x10e) = 0;
  *(undefined4 *)((int)this + 0x112) = 0;
  *(undefined4 *)((int)this + 0x116) = 0;
  BVar1 = PrintDlgA((LPPRINTDLGA)((int)this + 0xd8));
  if (BVar1 == 0) {
    uVar2 = 1;
  }
  else {
    local_10 = GlobalLock(*(HGLOBAL *)((int)this + 0xe0));
    if (*(int *)((int)this + 0xc0) == 1) {
      (local_10->field6_0x2c).field0.dmOrientation = *(short *)((int)this + 0x48);
    }
    else {
      (local_10->field6_0x2c).field0.dmOrientation = *(short *)((int)this + 0x4c);
    }
    local_10->dmFields = local_10->dmFields | 1;
    ResetDCA(*(HDC *)((int)this + 0xe8),local_10);
    GlobalUnlock(*(HGLOBAL *)((int)this + 0xe0));
    *(undefined4 *)((int)this + 0x10) = *(undefined4 *)((int)this + 0xe0);
    *(undefined4 *)((int)this + 0x14) = *(undefined4 *)((int)this + 0xe4);
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),8);
    *(int *)((int)this + 0x128) = iVar3;
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),10);
    *(int *)((int)this + 300) = iVar3;
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),4);
    *(int *)((int)this + 0x130) = iVar3;
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),6);
    *(int *)((int)this + 0x134) = iVar3;
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),0x58);
    *(int *)((int)this + 0x138) = iVar3;
    iVar3 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),0x5a);
    *(int *)((int)this + 0x13c) = iVar3;
    local_14 = GetDeviceCaps(*(HDC *)((int)this + 0xe8),0x70);
    local_c = GetDeviceCaps(*(HDC *)((int)this + 0xe8),0x71);
    *(int *)((int)this + 0x18) =
         (*(int *)((int)this + 0x8c) * *(int *)((int)this + 0x138)) / 1000 - local_14;
    *(int *)((int)this + 0x20) =
         (*(int *)((int)this + 0x94) * *(int *)((int)this + 0x138)) / 1000 - local_14;
    *(int *)((int)this + 0x1c) =
         (*(int *)((int)this + 0x90) * *(int *)((int)this + 0x13c)) / 1000 - local_c;
    *(int *)((int)this + 0x24) =
         (*(int *)((int)this + 0x98) * *(int *)((int)this + 0x13c)) / 1000 - local_c;
    if (*(int *)((int)this + 0xc0) == 1) {
      uVar2 = FUN_0041ac36(this);
    }
    else {
      *(double *)((int)this + 0x58) =
           ((double)*(int *)((int)this + 0x13c) * *(double *)((int)this + 0x50)) / 50.0;
      if ((DAT_00452efc == 0) || (DAT_00452efc == 1)) {
        uVar4 = FUN_0043ee30();
        local_1c = (int)uVar4;
        uVar4 = FUN_0043ee30();
        local_18 = (int)uVar4;
        if ((0x7ff8 < local_1c) || (0x7ff8 < local_18)) {
          if (local_1c < local_18) {
            *(double *)((int)this + 0x58) =
                 32760.0 / (double)*(int *)(*(int *)((int)this + 0xc) + 0x16a8);
          }
          else {
            *(double *)((int)this + 0x58) =
                 32760.0 / (double)*(int *)(*(int *)((int)this + 0xc) + 0x16a4);
          }
        }
        if ((DAT_00452ef4 != 0) && (DAT_00452efc == 0)) {
          FUN_0043ed39(local_a4,(byte *)0x44ca90);
          MessageBoxA(*(HWND *)this,local_a4,"",0);
        }
      }
      uVar2 = FUN_0041b260(this);
    }
  }
  return uVar2;
}
