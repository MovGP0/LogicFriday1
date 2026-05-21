/* 00410a47 FUN_00410a47 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 FUN_00410a47(undefined4 param_1,uint *param_2,char *param_3)

{
  uint uVar1;
  size_t sVar2;
  undefined4 uVar3;
  uint unaff_retaddr;
  uint local_190;
  wchar_t local_18c [180];
  uint local_24;
  uint local_20;
  uint local_1c;
  uint local_18;
  FILE *local_14;
  uint local_10;
  uint local_c;
  uint local_8;
  
  local_24 = DAT_00451a00 ^ unaff_retaddr;
  sVar2 = _strlen(param_3);
  if (sVar2 == 0) {
    uVar3 = 0x1d0008;
  }
  else {
    uVar1 = param_2[0x31];
    local_c = param_2[0x32];
    local_14 = (FILE *)FUN_0043e6f2(param_3,"wt");
    if (local_14 == (FILE *)0x0) {
      uVar3 = 0x2b0001;
    }
    else {
      FUN_0043ebd0((uint *)local_18c,(uint *)&DAT_0044ad26);
      for (local_18 = 0; local_18 < uVar1; local_18 = local_18 + 1) {
        FUN_0043ebe0((uint *)local_18c,(uint *)((int)param_2 + local_18 * 9 + 0x160));
        FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc4);
      }
      FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc4);
      for (local_18 = 0; local_18 < local_c - 1; local_18 = local_18 + 1) {
        FUN_0043ebe0((uint *)local_18c,(uint *)((int)param_2 + local_18 * 9 + 0xd0));
        FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc4);
      }
      FUN_0043ebe0((uint *)local_18c,(uint *)((int)param_2 + (local_c - 1) * 9 + 0xd0));
      FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044b734);
      FID_conflict__fwprintf(local_14,local_18c);
      if ((param_2[0x8f] == 0) || (param_2[0x91] == 0)) {
        local_1c = *param_2;
        for (local_18 = 0; local_18 < local_1c; local_18 = local_18 + 1) {
          FUN_0043ebd0((uint *)local_18c,(uint *)&DAT_0044ad26);
          local_190 = uVar1;
          while (local_190 = local_190 - 1, -1 < (int)local_190) {
            if ((local_18 & 1 << ((byte)local_190 & 0x1f)) == 0) {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb8);
            }
            else {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbbc);
            }
          }
          FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc4);
          for (local_190 = 0; (int)local_190 < (int)(local_c - 1); local_190 = local_190 + 1) {
            if (*(int *)(param_2[local_190 + 0x21] + local_18 * 4) == 1) {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbbc);
            }
            else if (*(int *)(param_2[local_190 + 0x21] + local_18 * 4) == 0) {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb8);
            }
            else {
              if (*(int *)(param_2[local_190 + 0x21] + local_18 * 4) != 2) {
                _fclose(local_14);
                return 0x1d0009;
              }
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc0);
            }
          }
          if (*(int *)(param_2[local_c + 0x20] + local_18 * 4) == 1) {
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb4);
          }
          else if (*(int *)(param_2[local_c + 0x20] + local_18 * 4) == 0) {
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb0);
          }
          else {
            if (*(int *)(param_2[local_c + 0x20] + local_18 * 4) != 2) {
              _fclose(local_14);
              return 0x1d0009;
            }
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044add0);
          }
          FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044b734);
          FID_conflict__fwprintf(local_14,local_18c);
        }
      }
      else {
        local_1c = param_2[0x7d];
        for (local_18 = 0; local_18 < local_1c; local_18 = local_18 + 1) {
          FUN_0043ebd0((uint *)local_18c,(uint *)&DAT_0044ad26);
          local_8 = *(uint *)(param_2[0x7e] + 8 + local_18 * 0xc);
          local_20 = *(uint *)(param_2[0x7e] + 4 + local_18 * 0xc);
          local_190 = uVar1;
          while (local_190 = local_190 - 1, -1 < (int)local_190) {
            local_10 = 1 << ((byte)local_190 & 0x1f);
            if ((local_8 & local_10) == 0) {
              if ((local_20 & local_10) == 0) {
                FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb8);
              }
              else {
                FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbbc);
              }
            }
            else {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc0);
            }
          }
          FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc4);
          for (local_190 = 0; (int)local_190 < (int)(local_c - 1); local_190 = local_190 + 1) {
            if (*(int *)(param_2[local_190 + 0x7f] + local_18 * 4) == 1) {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbbc);
            }
            else if (*(int *)(param_2[local_190 + 0x7f] + local_18 * 4) == 0) {
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb8);
            }
            else {
              if (*(int *)(param_2[local_190 + 0x7f] + local_18 * 4) != 2) {
                _fclose(local_14);
                return 0x1d0009;
              }
              FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbc0);
            }
          }
          if (*(int *)(param_2[local_c + 0x7e] + local_18 * 4) == 1) {
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb4);
          }
          else if (*(int *)(param_2[local_c + 0x7e] + local_18 * 4) == 0) {
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044bbb0);
          }
          else {
            if (*(int *)(param_2[local_c + 0x7e] + local_18 * 4) != 2) {
              _fclose(local_14);
              return 0x1d0009;
            }
            FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044add0);
          }
          FUN_0043ebe0((uint *)local_18c,(uint *)&DAT_0044b734);
          FID_conflict__fwprintf(local_14,local_18c);
        }
      }
      _fclose(local_14);
      uVar3 = 0;
    }
  }
  return uVar3;
}
