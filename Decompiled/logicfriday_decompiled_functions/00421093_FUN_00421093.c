/* 00421093 FUN_00421093 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __fastcall FUN_00421093(uint *param_1)

{
  bool bVar1;
  size_t sVar2;
  void *pvVar3;
  uint unaff_retaddr;
  int local_54;
  int local_4c;
  int local_48;
  int local_44;
  char local_3c [32];
  uint local_1c;
  uint local_18;
  int local_14;
  int local_10;
  int local_c;
  char *local_8;
  
  local_1c = DAT_00451a00 ^ unaff_retaddr;
  local_10 = param_1[0x31] - 1;
  local_c = 0;
  local_4c = 0;
  FUN_0043ed39((char *)param_1[0x9a],&DAT_0044cba0);
  if (param_1[0x90] == 0) {
    for (local_14 = 0; local_14 < (int)param_1[0x31]; local_14 = local_14 + 1) {
      sVar2 = _strlen((char *)((int)param_1 + local_14 * 9 + 0x160));
      local_4c = local_4c + sVar2;
    }
    local_c = local_4c + 1 + param_1[0x31] * 2;
    sVar2 = _strlen((char *)param_1[0x9a]);
    local_44 = sVar2 + 10;
    for (local_14 = 0; local_14 < (int)param_1[0x32]; local_14 = local_14 + 1) {
      local_44 = local_44 + (*param_1 - param_1[local_14 + 1]) * local_c;
    }
  }
  else {
    if (param_1[0x93] == 0) {
      return 0;
    }
    sVar2 = _strlen((char *)param_1[0x9c]);
    local_44 = sVar2 + 0x100;
  }
  if ((int)(param_1[0x597] * 0x7fff + -0x100) < local_44) {
    param_1[0x597] = local_44 / 0x7fff + 1;
    pvVar3 = _realloc((void *)param_1[0x9a],param_1[0x597] * 0x7fff);
    param_1[0x9a] = (uint)pvVar3;
  }
  if (param_1[0x90] == 0) {
    sVar2 = _strlen((char *)param_1[0x9a]);
    local_8 = (char *)(sVar2 + param_1[0x9a]);
    for (local_14 = 0; local_14 < (int)param_1[0x32]; local_14 = local_14 + 1) {
      FUN_0043ed39(local_3c,(byte *)"%s = ");
      for (local_48 = 0; sVar2 = _strlen(local_3c), local_48 < (int)sVar2; local_48 = local_48 + 1)
      {
        *local_8 = local_3c[local_48];
        local_8 = local_8 + 1;
      }
      if (*param_1 == param_1[local_14 + 1]) {
        *local_8 = '1';
        local_8[1] = ';';
        local_8[2] = '\n';
        local_8 = local_8 + 3;
        param_1[0x91] = (uint)(param_1[0x90] != 0);
      }
      else if (param_1[local_14 + 1] == 0) {
        *local_8 = '0';
        local_8[1] = ';';
        local_8[2] = '\n';
        local_8 = local_8 + 3;
        param_1[0x91] = (uint)(param_1[0x90] != 0);
      }
      else {
        for (local_18 = 0; local_18 < *param_1; local_18 = local_18 + 1) {
          if (*(int *)(param_1[local_14 + 0x21] + local_18 * 4) == 0) {
            *local_8 = '(';
            local_8 = local_8 + 1;
            bVar1 = false;
            for (local_54 = local_10; -1 < local_54; local_54 = local_54 + -1) {
              if (bVar1) {
                *local_8 = '+';
                local_8 = local_8 + 1;
              }
              bVar1 = true;
              sVar2 = _strlen((char *)((int)param_1 + (local_10 - local_54) * 9 + 0x160));
              for (local_48 = 0; local_48 < (int)sVar2; local_48 = local_48 + 1) {
                *local_8 = *(char *)((int)param_1 + local_48 + (local_10 - local_54) * 9 + 0x160);
                local_8 = local_8 + 1;
              }
              if ((local_18 & 1 << ((byte)local_54 & 0x1f)) != 0) {
                *local_8 = '\'';
                local_8 = local_8 + 1;
              }
            }
            *local_8 = ')';
            local_8 = local_8 + 1;
          }
        }
        *local_8 = ';';
        local_8[1] = '\n';
        local_8 = local_8 + 2;
      }
    }
    if (local_8 != (char *)0x0) {
      *local_8 = '\n';
      local_8[1] = '\0';
      local_8 = local_8 + 2;
    }
    FUN_004219f6(param_1,(uint *)param_1[0x9a]);
  }
  else if (param_1[0x93] != 0) {
    FUN_0043ebd0((uint *)param_1[0x9a],(uint *)"Minimized Product of Sums:\n");
    FUN_0043ebe0((uint *)param_1[0x9a],(uint *)param_1[0x9c]);
    FUN_0043ebe0((uint *)param_1[0x9a],(uint *)&DAT_0044b734);
    FUN_004219f6(param_1,(uint *)param_1[0x9a]);
  }
  return 0;
}
