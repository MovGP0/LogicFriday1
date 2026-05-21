/* 00422090 FUN_00422090 */

undefined4 __thiscall FUN_00422090(void *this,HWND param_1,int param_2,short param_3)

{
  int nIDButton;
  bool bVar1;
  bool bVar2;
  bool bVar3;
  bool bVar4;
  bool bVar5;
  bool bVar6;
  bool bVar7;
  bool bVar8;
  UINT UVar9;
  FILE *_File;
  int local_38;
  int local_30;
  int local_2c;
  
  if (param_2 == 0x110) {
    local_2c = *(int *)((int)this + 0x3a8);
    while (local_2c = local_2c + -1, -1 < local_2c) {
      if (*(int *)((int)this + local_2c * 0x118 + 0x3cc) != 0) {
        CheckDlgButton(param_1,*(int *)((int)this + local_2c * 0x118 + 0x3b8),1);
      }
    }
    CheckRadioButton(param_1,0x403,0x405,*(int *)((int)this + 0x3b4));
    return 1;
  }
  if (param_2 == 0x111) {
    if (param_3 == 1) {
      *(undefined4 *)((int)this + 0x3ac) = 1;
      bVar8 = false;
      bVar7 = false;
      bVar5 = false;
      bVar6 = false;
      bVar3 = false;
      bVar4 = false;
      bVar2 = false;
      bVar1 = false;
      local_30 = *(int *)((int)this + 0x3a8);
      while (local_30 = local_30 + -1, -1 < local_30) {
        nIDButton = *(int *)((int)this + local_30 * 0x118 + 0x3b8);
        UVar9 = IsDlgButtonChecked(param_1,nIDButton);
        if (UVar9 == 1) {
          *(undefined4 *)((int)this + local_30 * 0x118 + 0x3cc) = 1;
          if (((nIDButton == 0x3f8) || (nIDButton == 0x3f9)) || (nIDButton == 0x3fa)) {
            bVar2 = true;
            if (nIDButton == 0x3f8) {
              bVar3 = true;
              *(undefined4 *)((int)this + 0x3ac) = 0;
            }
            else if (nIDButton == 0x3f9) {
              bVar5 = true;
            }
            else if (nIDButton == 0x3fa) {
              bVar8 = true;
            }
          }
          if (((nIDButton == 0x3f5) || (nIDButton == 0x3f6)) || (nIDButton == 0x3f7)) {
            bVar1 = true;
            if (nIDButton == 0x3f5) {
              bVar4 = true;
            }
            else if (nIDButton == 0x3f6) {
              bVar6 = true;
            }
            else if (nIDButton == 0x3f7) {
              bVar7 = true;
            }
          }
        }
        else {
          *(undefined4 *)((int)this + local_30 * 0x118 + 0x3cc) = 0;
        }
      }
      if ((!bVar1) && (!bVar2)) {
        MessageBoxA(*(HWND *)((int)this + 0x16f0),
                    "You must include either a NAND or a NOR gate type.","Gate Selection",0);
        return 1;
      }
      UVar9 = IsDlgButtonChecked(param_1,0x403);
      if (UVar9 == 1) {
        *(undefined4 *)((int)this + 0x3b4) = 0x403;
      }
      else {
        UVar9 = IsDlgButtonChecked(param_1,0x404);
        if (UVar9 == 1) {
          *(undefined4 *)((int)this + 0x3b4) = 0x404;
        }
        else {
          UVar9 = IsDlgButtonChecked(param_1,0x405);
          if (UVar9 == 1) {
            *(undefined4 *)((int)this + 0x3b4) = 0x405;
          }
        }
      }
      FUN_0043ebd0((uint *)((int)this + 0x18e8),(uint *)((int)this + 0x20e8));
      if ((!bVar3) && (!bVar4)) {
        if (bVar5) {
          FUN_0043ebe0((uint *)((int)this + 0x18e8),
                       (uint *)((int)this + *(int *)((int)this + 0x22f8) * 0x118 + 0x3d0));
          *(undefined4 *)((int)this + 0x3ac) = 0;
        }
        else if (bVar6) {
          FUN_0043ebe0((uint *)((int)this + 0x18e8),
                       (uint *)((int)this + *(int *)((int)this + 0x22ec) * 0x118 + 0x3d0));
          *(undefined4 *)((int)this + 0x3ac) = 1;
        }
        else if (bVar2) {
          FUN_0043ebe0((uint *)((int)this + 0x18e8),
                       (uint *)((int)this + *(int *)((int)this + 0x22f8) * 0x118 + 0x3d0));
          *(undefined4 *)((int)this + 0x3ac) = 0;
        }
        else {
          FUN_0043ebe0((uint *)((int)this + 0x18e8),
                       (uint *)((int)this + *(int *)((int)this + 0x22ec) * 0x118 + 0x3d0));
          *(undefined4 *)((int)this + 0x3ac) = 1;
        }
      }
      if ((bVar7) && (!bVar6)) {
        FUN_0043ebe0((uint *)((int)this + 0x18e8),
                     (uint *)((int)this + *(int *)((int)this + 0x22f0) * 0x118 + 0x3d0));
      }
      if ((bVar8) && (!bVar5)) {
        FUN_0043ebe0((uint *)((int)this + 0x18e8),
                     (uint *)((int)this + *(int *)((int)this + 0x22fc) * 0x118 + 0x3d0));
      }
      local_38 = *(int *)((int)this + 0x3a8);
      while (local_38 = local_38 + -1, -1 < local_38) {
        if (*(int *)((int)this + local_38 * 0x118 + 0x3cc) != 0) {
          FUN_0043ebe0((uint *)((int)this + 0x18e8),(uint *)((int)this + local_38 * 0x118 + 0x3d0));
        }
      }
      _File = (FILE *)FUN_0043e6f2((char *)((int)this + 0x21e8),"wt");
      if (_File == (FILE *)0x0) {
        return 0x2f0011;
      }
      FID_conflict__fwprintf(_File,(wchar_t *)((int)this + 0x18e8));
      _fclose(_File);
      EndDialog(param_1,1);
      return 1;
    }
    if (param_3 == 2) {
      EndDialog(param_1,0);
      return 1;
    }
    if (param_3 == 0x428) {
      SendMessageA(*(HWND *)((int)this + 0x16f0),0x111,0x8019,0);
      return 1;
    }
  }
  return 0;
}
