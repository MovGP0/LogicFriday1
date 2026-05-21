/* 004164b1 FUN_004164b1 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004164b1(void *this,HWND param_1,int param_2,short param_3)

{
  uint unaff_retaddr;
  char local_40c [1028];
  uint local_8;
  
  local_8 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 0x110) {
    if (*(int *)((int)this + 0x200) == 3) {
      FUN_0043ed39(local_40c,(byte *)"%s equals %s.");
    }
    else if (*(int *)((int)this + 0x200) == 5) {
      if (*(int *)((int)this + 0x210) == 1) {
        FUN_0043ed39(local_40c,(byte *)"%s is a subset of %s.");
      }
      else {
        FUN_0043ed39(local_40c,(byte *)"%s is a subset of %s.");
      }
    }
    else if (*(int *)((int)this + 0x200) == 6) {
      FUN_0043ed39(local_40c,(byte *)"%s is the complement of %s.");
    }
    else if (*(int *)((int)this + 0x200) == 4) {
      FUN_0043ed39(local_40c,(byte *)"%s is not equal to %s.");
    }
    SetDlgItemTextA(param_1,0x410,local_40c);
    FUN_0043ed39(local_40c,(byte *)"%s and %s:");
    SetDlgItemTextA(param_1,0x411,local_40c);
    FUN_0043ed39(local_40c,(byte *)"%s only:");
    SetDlgItemTextA(param_1,0x412,local_40c);
    FUN_0043ed39(local_40c,(byte *)"%s only:");
    SetDlgItemTextA(param_1,0x413,local_40c);
    FUN_0043ed39(local_40c,&DAT_0044b960);
    SetDlgItemTextA(param_1,0x414,local_40c);
    FUN_0043ed39(local_40c,&DAT_0044b960);
    SetDlgItemTextA(param_1,0x415,local_40c);
    FUN_0043ed39(local_40c,&DAT_0044b960);
    SetDlgItemTextA(param_1,0x416,local_40c);
    if (*(int *)((int)this + 0x214) == 0) {
      SetDlgItemTextA(param_1,0x417,"");
    }
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      EndDialog(param_1,0);
      return 1;
    }
    if (param_3 == 0x40e) {
      EndDialog(param_1,1);
      return 1;
    }
  }
  return 0;
}
