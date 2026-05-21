/* 004170c1 FUN_004170c1 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004170c1(void *this,HWND param_1,int param_2,uint param_3,uint *param_4)

{
  bool bVar1;
  uint uVar2;
  UINT UVar3;
  undefined3 extraout_var;
  undefined3 extraout_var_00;
  undefined3 extraout_var_01;
  HWND pHVar4;
  uint unaff_retaddr;
  BOOL BVar5;
  char local_94 [132];
  uint local_10;
  uint local_c;
  int local_8;
  
  local_10 = DAT_00451a00 ^ unaff_retaddr;
  if (param_2 == 0x110) {
    DAT_0046c514 = param_4;
    CheckRadioButton(param_1,0x419,0x41c,param_4[1]);
    CheckRadioButton(param_1,0x420,0x423,*DAT_0046c514);
    SendDlgItemMessageA(param_1,0x41d,0xc5,3,0);
    PostMessageA(param_1,0x111,DAT_0046c514[1] & 0xffff,0);
    PostMessageA(param_1,0x111,*DAT_0046c514 & 0xffff,0);
    return 1;
  }
  if (param_2 != 0x111) {
    return 0;
  }
  uVar2 = param_3 & 0xffff;
  if (0x41b < uVar2) {
    if (uVar2 == 0x41c) {
      BVar5 = 1;
      pHVar4 = GetDlgItem(param_1,0x41d);
      EnableWindow(pHVar4,BVar5);
      SetDlgItemInt(param_1,0x41d,DAT_0046c514[2],0);
      return 1;
    }
    if (uVar2 == 0x41d) {
      if (param_3 >> 0x10 == 0x300) {
        local_c = GetDlgItemInt(param_1,0x41d,(BOOL *)0x0,0);
        if (local_c < DAT_0046c514[3]) {
          BVar5 = 0;
          pHVar4 = GetDlgItem(param_1,0x423);
          EnableWindow(pHVar4,BVar5);
        }
        else {
          BVar5 = 1;
          pHVar4 = GetDlgItem(param_1,0x423);
          EnableWindow(pHVar4,BVar5);
        }
      }
    }
    else if (uVar2 != 0x428) {
      return 0;
    }
    SendMessageA(*(HWND *)this,0x111,0x801a,0);
    return 1;
  }
  if (uVar2 < 0x419) {
    if (uVar2 == 1) {
      UVar3 = IsDlgButtonChecked(param_1,0x419);
      if (UVar3 == 0) {
        UVar3 = IsDlgButtonChecked(param_1,0x408);
        if (UVar3 == 0) {
          UVar3 = IsDlgButtonChecked(param_1,0x41a);
          if (UVar3 == 0) {
            UVar3 = IsDlgButtonChecked(param_1,0x41b);
            if (UVar3 == 0) {
              UVar3 = IsDlgButtonChecked(param_1,0x41c);
              if (UVar3 != 0) {
                local_c = GetDlgItemInt(param_1,0x41d,(BOOL *)0x0,0);
                bVar1 = FUN_004175ae(this,local_c,DAT_0046c514[3]);
                if (CONCAT31(extraout_var_01,bVar1) == 0) {
                  return 1;
                }
                if ((local_c < 8) || (0x40 < local_c)) {
                  SetDlgItemInt(param_1,0x41d,8,0);
                  MessageBoxA(*(HWND *)this,
                              "The length of unsigned int may not be less than 8 bits or more than 64 bits."
                              ,"Invalid Entry",0);
                  return 1;
                }
                if (local_c % 8 != 0) {
                  FUN_0043ed39(local_94,(byte *)
                                        "Are you sure you want to set the length of <unsigned int> to %d?"
                              );
                  local_8 = MessageBoxA(*(HWND *)this,local_94,"Verify Entry",0x23);
                  if (local_8 != 6) {
                    return 1;
                  }
                }
                DAT_0046c514[1] = 0x41c;
                DAT_0046c514[2] = local_c;
              }
            }
            else {
              DAT_0046c514[1] = 0x41b;
            }
          }
          else {
            DAT_0046c514[1] = 0x41a;
          }
        }
        else {
          DAT_0046c514[1] = 0x408;
          bVar1 = FUN_004175ae(this,0x10,DAT_0046c514[3]);
          if (CONCAT31(extraout_var_00,bVar1) == 0) {
            return 1;
          }
        }
      }
      else {
        DAT_0046c514[1] = 0x419;
        bVar1 = FUN_004175ae(this,8,DAT_0046c514[3]);
        if (CONCAT31(extraout_var,bVar1) == 0) {
          return 1;
        }
      }
      UVar3 = IsDlgButtonChecked(param_1,0x420);
      if (UVar3 == 0) {
        *DAT_0046c514 = 0x423;
      }
      else {
        *DAT_0046c514 = 0x420;
      }
      EndDialog(param_1,1);
      return 1;
    }
    if (uVar2 == 2) {
      EndDialog(param_1,2);
      return 1;
    }
    if (uVar2 != 0x408) {
      return 0;
    }
  }
  BVar5 = 0;
  pHVar4 = GetDlgItem(param_1,0x41d);
  EnableWindow(pHVar4,BVar5);
  if (((param_3 & 0xffff) == 0x419) && (8 < DAT_0046c514[3])) {
    BVar5 = 0;
    pHVar4 = GetDlgItem(param_1,0x423);
    EnableWindow(pHVar4,BVar5);
  }
  else if (((param_3 & 0xffff) == 0x408) && (0x10 < DAT_0046c514[3])) {
    BVar5 = 0;
    pHVar4 = GetDlgItem(param_1,0x423);
    EnableWindow(pHVar4,BVar5);
  }
  else {
    BVar5 = 1;
    pHVar4 = GetDlgItem(param_1,0x423);
    EnableWindow(pHVar4,BVar5);
  }
  return 1;
}
