/* 00418362 FUN_00418362 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00418362(void *this,HWND param_1,int param_2,uint param_3,HWND param_4)

{
  uint uVar1;
  UINT UVar2;
  size_t sVar3;
  void *pvVar4;
  HWND pHVar5;
  int iVar6;
  char *pcVar7;
  undefined4 *puVar8;
  uint unaff_retaddr;
  BOOL bEnable;
  uint local_1b0;
  int local_1a4;
  undefined1 local_194 [8];
  undefined4 local_18c;
  undefined1 *local_180;
  undefined4 local_17c;
  undefined1 local_16c [8];
  undefined4 local_164;
  undefined1 *local_158;
  undefined4 local_154;
  undefined1 local_144 [8];
  undefined4 local_13c;
  char *local_130;
  undefined1 local_11c [8];
  undefined4 local_114;
  char *local_108;
  undefined1 local_f4 [8];
  undefined4 local_ec;
  int local_e0;
  undefined1 local_cc [8];
  undefined4 local_c4;
  int local_b8;
  undefined1 local_a4 [8];
  undefined4 local_9c;
  int local_90;
  char local_7c [32];
  uint local_5c;
  HWND local_58;
  undefined4 local_54;
  undefined4 local_50;
  int local_4c;
  undefined1 local_48 [5];
  undefined4 uStack_43;
  undefined4 local_3c;
  uint local_20;
  HWND local_1c;
  HWND local_18;
  HWND local_14;
  WPARAM local_10;
  HWND local_c;
  HWND local_8;
  
  local_20 = DAT_00451a00 ^ unaff_retaddr;
  local_54 = 1;
  local_50 = 0;
  local_4c = 0;
  local_48[0] = '\0';
  local_48._1_4_ = 0;
  uStack_43 = 0;
  pcVar7 = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
  puVar8 = &local_3c;
  for (iVar6 = 6; iVar6 != 0; iVar6 = iVar6 + -1) {
    *puVar8 = *(undefined4 *)pcVar7;
    pcVar7 = pcVar7 + 4;
    puVar8 = puVar8 + 1;
  }
  *(undefined2 *)puVar8 = *(undefined2 *)pcVar7;
  *(char *)((int)puVar8 + 2) = pcVar7[2];
  if (param_2 == 0x4e) {
    local_1c = param_4;
    local_c = param_4;
    if ((param_4->unused == *(int *)((int)this + 0x18)) ||
       (param_4->unused == *(int *)((int)this + 0x1c))) {
      local_8 = (HWND)param_4->unused;
      if (local_8 == *(HWND *)((int)this + 0x18)) {
        local_1a4 = *(int *)((int)this + 0xe4);
      }
      else {
        local_1a4 = *(int *)((int)this + 0xec);
      }
      local_4c = local_1a4;
      uVar1 = param_4[2].unused;
      if (uVar1 == 0xffffff96) {
        if (param_4[8].unused == 0) {
          return 0;
        }
        local_5c = FUN_004194bc(this,(char *)param_4[8].unused);
        if (local_5c != 0) {
          if ((int)local_5c < 0x65) {
            FUN_0041a18a(param_1,local_5c);
          }
          else {
            FUN_0040a274(*(HWND *)((int)this + 0xc),local_5c);
          }
          return 0;
        }
        *(int *)((int)this + 0x44) = local_c[4].unused;
        local_ec = 0;
        local_e0 = local_c[8].unused;
        SendMessageA(local_8,0x102e,local_c[4].unused,(LPARAM)local_f4);
        return 1;
      }
      if ((0xfffffffc < uVar1) && (uVar1 != 0xffffffff)) {
        local_58 = param_4;
        local_10 = param_4[3].unused;
        if ((local_10 != 0xffffffff) && ((int)local_10 < local_1a4)) {
          local_18 = (HWND)SendMessageA(local_8,0x1017,local_10,0);
          SendMessageA(local_18,0xc5,8,0);
        }
      }
    }
  }
  else {
    if (param_2 == 0x110) {
      if (DAT_00452ec4 != 0) {
        *(undefined4 *)((int)this + 0xf8) = 0;
        SendMessageA(*(HWND *)((int)this + 0xc),0x111,0x800a,(int)this + 0xf8);
        if (*(int *)((int)this + 0xf8) != 0) {
          *(undefined4 *)((int)this + 0xdc) = *(undefined4 *)(*(int *)((int)this + 0xf8) + 0xc4);
          *(undefined4 *)((int)this + 0xe0) = *(undefined4 *)(*(int *)((int)this + 0xf8) + 200);
        }
        bEnable = 0;
        pHVar5 = GetDlgItem(param_1,0x3ec);
        EnableWindow(pHVar5,bEnable);
      }
      *(undefined4 *)((int)this + 0x98) = 0;
      *(undefined4 *)((int)this + 0x94) = 0;
      *(undefined4 *)((int)this + 0xec) = 0;
      *(undefined4 *)((int)this + 0xe4) = 0;
      local_14 = param_4;
      *(int *)((int)this + 0xf4) = param_4->unused;
      SendDlgItemMessageA(param_1,0x3ed,0x465,0,0x20010);
      SendDlgItemMessageA(param_1,0x3ed,0x467,0,*(uint *)((int)this + 0xdc) & 0xffff);
      SendDlgItemMessageA(param_1,0x3f0,0x465,0,0x10010);
      SendDlgItemMessageA(param_1,0x3f0,0x467,0,*(uint *)((int)this + 0xe0) & 0xffff);
      pHVar5 = GetDlgItem(param_1,0x3ee);
      *(HWND *)((int)this + 0x18) = pHVar5;
      SendMessageA(*(HWND *)((int)this + 0x18),0x1036,0,0x21);
      *(undefined4 *)((int)this + 0x20) = 6;
      *(undefined **)((int)this + 0x2c) = &DAT_0044c988;
      *(undefined4 *)((int)this + 0x28) = 100;
      SendMessageA(*(HWND *)((int)this + 0x18),0x101b,0,(int)this + 0x20);
      *(undefined **)((int)this + 0x2c) = &DAT_0044c984;
      *(undefined4 *)((int)this + 0x28) = 0x19;
      SendMessageA(*(HWND *)((int)this + 0x18),0x101b,1,(int)this + 0x20);
      SendMessageA(*(HWND *)((int)this + 0x18),0x103a,2,(LPARAM)&local_54);
      *(undefined4 *)((int)this + 0x94) = 1;
      SendMessageA(param_1,0x111,0x30003ec,0);
      pHVar5 = GetDlgItem(param_1,0x406);
      *(HWND *)((int)this + 0x1c) = pHVar5;
      SendMessageA(*(HWND *)((int)this + 0x1c),0x1036,0,0x21);
      *(undefined4 *)((int)this + 0x20) = 6;
      *(undefined **)((int)this + 0x2c) = &DAT_0044c988;
      *(undefined4 *)((int)this + 0x28) = 100;
      SendMessageA(*(HWND *)((int)this + 0x1c),0x101b,0,(int)this + 0x20);
      *(undefined **)((int)this + 0x2c) = &DAT_0044c984;
      *(undefined4 *)((int)this + 0x28) = 0x19;
      SendMessageA(*(HWND *)((int)this + 0x1c),0x101b,1,(int)this + 0x20);
      SendMessageA(*(HWND *)((int)this + 0x1c),0x103a,2,(LPARAM)&local_54);
      *(undefined4 *)((int)this + 0x98) = 1;
      SendMessageA(param_1,0x111,0x30003ef,0);
      if (DAT_00452ec4 != 0) {
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)((int)this + 0xdc)) {
          local_9c = 0;
          local_90 = *(int *)((int)this + 0xf8) + 0x160 + *(int *)((int)this + 0xf0) * 9;
          SendMessageA(*(HWND *)((int)this + 0x18),0x102e,*(WPARAM *)((int)this + 0xf0),
                       (LPARAM)local_a4);
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)((int)this + 0xe0)) {
          local_c4 = 0;
          local_b8 = *(int *)((int)this + 0xf8) + 0xd0 + *(int *)((int)this + 0xf0) * 9;
          SendMessageA(*(HWND *)((int)this + 0x1c),0x102e,*(WPARAM *)((int)this + 0xf0),
                       (LPARAM)local_cc);
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
      }
      return 1;
    }
    if (param_2 == 0x111) {
      uVar1 = param_3 & 0xffff;
      if (uVar1 == 1) {
        pHVar5 = GetDlgItem(param_1,1);
        if (param_4 != pHVar5) {
          return 1;
        }
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)(*(int *)((int)this + 0xf4) + 200)) {
          if (*(int *)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4) != 0) {
            _free(*(void **)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4));
            *(undefined4 *)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4) = 0;
          }
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
        _memset(*(void **)((int)this + 0xf4),0,0x1f0);
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)((int)this + 0xe0)) {
          local_164 = 0;
          local_154 = 9;
          local_158 = local_48;
          SendMessageA(*(HWND *)((int)this + 0x1c),0x102d,*(WPARAM *)((int)this + 0xf0),
                       (LPARAM)local_16c);
          FUN_0043ebd0((uint *)(*(int *)((int)this + 0xf4) + 0xd0 + *(int *)((int)this + 0xf0) * 9),
                       (uint *)local_48);
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
        *(undefined4 *)(*(int *)((int)this + 0xf4) + 200) = *(undefined4 *)((int)this + 0xe0);
        *(undefined4 *)(*(int *)((int)this + 0xf4) + 0xcc) = 0;
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)((int)this + 0xdc)) {
          local_18c = 0;
          local_17c = 9;
          local_180 = local_48;
          SendMessageA(*(HWND *)((int)this + 0x18),0x102d,*(WPARAM *)((int)this + 0xf0),
                       (LPARAM)local_194);
          FUN_0043ebd0((uint *)(*(int *)((int)this + 0xf4) + 0x160 + *(int *)((int)this + 0xf0) * 9)
                       ,(uint *)local_48);
          sVar3 = _strlen(local_48);
          if (1 < sVar3) {
            *(undefined4 *)(*(int *)((int)this + 0xf4) + 0xcc) = 1;
          }
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
        *(undefined4 *)(*(int *)((int)this + 0xf4) + 0xc4) = *(undefined4 *)((int)this + 0xdc);
        **(int **)((int)this + 0xf4) = 1 << ((byte)*(undefined4 *)((int)this + 0xdc) & 0x1f);
        *(undefined4 *)((int)this + 0xf0) = 0;
        while (*(uint *)((int)this + 0xf0) < *(uint *)((int)this + 0xe0)) {
          pvVar4 = _realloc(*(void **)(*(int *)((int)this + 0xf4) + 0x84 +
                                      *(int *)((int)this + 0xf0) * 4),
                            **(int **)((int)this + 0xf4) << 2);
          *(void **)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4) = pvVar4;
          if (*(int *)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4) == 0) {
            EndDialog(param_1,0x40012);
          }
          _memset(*(void **)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4),0,
                  **(int **)((int)this + 0xf4) << 2);
          *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
        }
        if (DAT_00452ec4 != 0) {
          if (*(uint *)(*(int *)((int)this + 0xf8) + 200) < *(uint *)((int)this + 0xe0)) {
            local_1b0 = *(uint *)(*(int *)((int)this + 0xf8) + 200);
          }
          else {
            local_1b0 = *(uint *)((int)this + 0xe0);
          }
          *(undefined4 *)((int)this + 0xf0) = 0;
          while (*(uint *)((int)this + 0xf0) < local_1b0) {
            *(undefined4 *)(*(int *)((int)this + 0xf4) + 4 + *(int *)((int)this + 0xf0) * 4) =
                 *(undefined4 *)(*(int *)((int)this + 0xf8) + 4 + *(int *)((int)this + 0xf0) * 4);
            *(undefined4 *)(*(int *)((int)this + 0xf4) + 0x44 + *(int *)((int)this + 0xf0) * 4) =
                 *(undefined4 *)(*(int *)((int)this + 0xf8) + 0x44 + *(int *)((int)this + 0xf0) * 4)
            ;
            _memcpy(*(void **)(*(int *)((int)this + 0xf4) + 0x84 + *(int *)((int)this + 0xf0) * 4),
                    *(void **)(*(int *)((int)this + 0xf8) + 0x84 + *(int *)((int)this + 0xf0) * 4),
                    **(int **)((int)this + 0xf4) << 2);
            *(int *)((int)this + 0xf0) = *(int *)((int)this + 0xf0) + 1;
          }
        }
        *(undefined4 *)((int)this + 0x98) = 0;
        *(undefined4 *)((int)this + 0x94) = 0;
        EndDialog(param_1,1);
        return 1;
      }
      if (uVar1 == 2) {
        pHVar5 = GetDlgItem(param_1,2);
        if (param_4 != pHVar5) {
          return 1;
        }
        *(undefined4 *)((int)this + 0x98) = 0;
        *(undefined4 *)((int)this + 0x94) = 0;
        while (*(int *)((int)this + 0xec) != 0) {
          *(int *)((int)this + 0xec) = *(int *)((int)this + 0xec) + -1;
          *(int *)((int)this + 0xe8) = *(int *)((int)this + 0xe8) + -1;
        }
        EndDialog(param_1,0);
        return 1;
      }
      if (uVar1 == 0x3ec) {
        if ((param_3 >> 0x10 == 0x300) && (*(int *)((int)this + 0x94) != 0)) {
          UVar2 = GetDlgItemInt(param_1,0x3ec,(BOOL *)0x0,0);
          *(UINT *)((int)this + 0xdc) = UVar2;
          while (*(uint *)((int)this + 0xe4) < *(uint *)((int)this + 0xdc)) {
            *(undefined4 *)((int)this + 0x44) = *(undefined4 *)((int)this + 0xe4);
            *(undefined4 *)((int)this + 0x48) = 0;
            *(undefined4 *)((int)this + 0x58) = 9;
            *(undefined4 *)((int)this + 0x40) = 1;
            FUN_0043ed39(local_48,&DAT_0044c980);
            *(undefined1 **)((int)this + 0x54) = local_48;
            SendMessageA(*(HWND *)((int)this + 0x18),0x1007,0,(int)this + 0x40);
            FUN_0043ed39(local_7c,&DAT_0044b960);
            local_114 = 1;
            local_108 = local_7c;
            SendMessageA(*(HWND *)((int)this + 0x18),0x102e,*(WPARAM *)((int)this + 0xe4),
                         (LPARAM)local_11c);
            *(int *)((int)this + 0xe4) = *(int *)((int)this + 0xe4) + 1;
          }
          while (*(uint *)((int)this + 0xdc) < *(uint *)((int)this + 0xe4)) {
            SendMessageA(*(HWND *)((int)this + 0x18),0x1008,*(int *)((int)this + 0xe4) - 1,0);
            *(int *)((int)this + 0xe4) = *(int *)((int)this + 0xe4) + -1;
          }
          return 1;
        }
      }
      else if (uVar1 == 0x3ef) {
        if ((param_3 >> 0x10 == 0x300) && (*(int *)((int)this + 0x98) != 0)) {
          UVar2 = GetDlgItemInt(param_1,0x3ef,(BOOL *)0x0,0);
          *(UINT *)((int)this + 0xe0) = UVar2;
          while (*(uint *)((int)this + 0xec) < *(uint *)((int)this + 0xe0)) {
            *(undefined4 *)((int)this + 0x44) = *(undefined4 *)((int)this + 0xec);
            *(undefined4 *)((int)this + 0x48) = 0;
            *(undefined4 *)((int)this + 0x58) = 9;
            *(undefined4 *)((int)this + 0x40) = 1;
            FUN_0043ed39(local_48,&DAT_0044a700);
            iVar6 = FUN_0040be9e(local_48);
            if (iVar6 != 0) {
              iVar6 = FUN_0040be0b();
              *(int *)((int)this + 0xe8) = iVar6;
              FUN_0043ed39(local_48,&DAT_0044a700);
            }
            *(undefined1 **)((int)this + 0x54) = local_48;
            SendMessageA(*(HWND *)((int)this + 0x1c),0x1007,0,(int)this + 0x40);
            FUN_0043ed39(local_7c,&DAT_0044b960);
            local_13c = 1;
            local_130 = local_7c;
            SendMessageA(*(HWND *)((int)this + 0x1c),0x102e,*(WPARAM *)((int)this + 0xec),
                         (LPARAM)local_144);
            *(int *)((int)this + 0xec) = *(int *)((int)this + 0xec) + 1;
            *(int *)((int)this + 0xe8) = *(int *)((int)this + 0xe8) + 1;
          }
          while (*(uint *)((int)this + 0xe0) < *(uint *)((int)this + 0xec)) {
            SendMessageA(*(HWND *)((int)this + 0x1c),0x1008,*(int *)((int)this + 0xec) - 1,0);
            *(int *)((int)this + 0xec) = *(int *)((int)this + 0xec) + -1;
            *(int *)((int)this + 0xe8) = *(int *)((int)this + 0xe8) + -1;
          }
          return 1;
        }
      }
      else if (uVar1 == 0x428) {
        SendMessageA(*(HWND *)((int)this + 0xc),0x111,0x8018,0);
        return 1;
      }
    }
  }
  return 0;
}
