/* 00411871 FUN_00411871 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int FUN_00411871(uint *param_1,int param_2)

{
  size_t sVar1;
  int iVar2;
  uint unaff_retaddr;
  uint *local_1a4;
  uint local_1a0;
  uint local_19c [90];
  uint local_34;
  int local_30;
  char local_2c [7];
  char local_25;
  char *local_24;
  uint local_20;
  uint local_1c [4];
  uint local_c;
  uint *local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  local_8 = (uint *)0x0;
  local_24 = (char *)0x0;
  local_1a4 = (uint *)0x0;
  local_2c[0] = ',';
  local_2c[1] = '\t';
  local_2c[2] = 0;
  local_30 = 0;
  local_20 = 0;
  local_1a0 = 0;
  FUN_0043ebd0(local_19c,param_1);
  local_1a4 = local_19c;
  local_8 = (uint *)FUN_00415979(&local_1a4,local_2c);
  if (local_8 == (uint *)0x0) {
    iVar2 = 9;
  }
  else {
    while (sVar1 = _strlen((char *)local_8), sVar1 != 0) {
      sVar1 = _strlen((char *)local_8);
      if (8 < sVar1) {
        return 3;
      }
      local_34 = FUN_0040daf0((char *)local_8);
      if (local_34 != 0) {
        return (local_34 & 0xffff) + 0x60000;
      }
      FUN_0043ebd0((uint *)(param_2 + 0x160 + local_20 * 9),local_8);
      local_20 = local_20 + 1;
      local_8 = (uint *)FUN_00415979(&local_1a4,local_2c);
    }
    if (local_20 == *(int *)(param_2 + 0xc4)) {
      local_20 = 0;
      local_8 = (uint *)FUN_00415979(&local_1a4,local_2c);
      while (local_8 != (uint *)0x0) {
        sVar1 = _strlen((char *)local_8);
        if (9 < sVar1) {
          return 3;
        }
        FUN_0043ebd0(local_1c,local_8);
        local_24 = _strrchr((char *)local_1c,10);
        if ((local_24 != (char *)0x0) ||
           (local_24 = _strrchr((char *)local_1c,0xd), local_24 != (char *)0x0)) {
          *local_24 = '\0';
        }
        sVar1 = _strlen((char *)local_1c);
        if (8 < sVar1) {
          return 3;
        }
        local_34 = FUN_0040daf0((char *)local_1c);
        if (local_34 != 0) {
          return (local_34 & 0xffff) + 0x60000;
        }
        FUN_0043ebd0((uint *)(param_2 + 0xd0 + local_20 * 9),local_1c);
        local_20 = local_20 + 1;
        local_8 = (uint *)FUN_00415979(&local_1a4,local_2c);
      }
      if (local_20 == *(int *)(param_2 + 200)) {
        for (local_20 = 0; local_20 < *(uint *)(param_2 + 0xc4); local_20 = local_20 + 1) {
          local_25 = *(char *)(param_2 + 0x160 + local_20 * 9);
          if ((((local_25 != '0') && (local_25 != '1')) && (local_25 != 'X')) && (local_25 != 'x'))
          {
            local_30 = 1;
            break;
          }
        }
        if (local_30 == 0) {
          iVar2 = 2;
        }
        else {
          for (local_20 = 0; local_1a0 = local_20, local_20 < *(int *)(param_2 + 0xc4) - 1U;
              local_20 = local_20 + 1) {
            while (local_1a0 = local_1a0 + 1, local_1a0 < *(uint *)(param_2 + 0xc4)) {
              iVar2 = _strcmp((char *)(param_2 + 0x160 + local_20 * 9),
                              (char *)(param_2 + 0x160 + local_1a0 * 9));
              if (iVar2 == 0) {
                return 5;
              }
            }
          }
          for (local_20 = 0; local_20 < *(uint *)(param_2 + 0xc4); local_20 = local_20 + 1) {
            for (local_1a0 = 0; local_1a0 < *(uint *)(param_2 + 200); local_1a0 = local_1a0 + 1) {
              iVar2 = _strcmp((char *)(param_2 + 0x160 + local_20 * 9),
                              (char *)(param_2 + 0xd0 + local_1a0 * 9));
              if (iVar2 == 0) {
                return 5;
              }
            }
          }
          for (local_20 = 0; local_1a0 = local_20, local_20 < *(int *)(param_2 + 200) - 1U;
              local_20 = local_20 + 1) {
            while (local_1a0 = local_1a0 + 1, local_1a0 < *(uint *)(param_2 + 200)) {
              iVar2 = _strcmp((char *)(param_2 + 0xd0 + local_20 * 9),
                              (char *)(param_2 + 0xd0 + local_1a0 * 9));
              if (iVar2 == 0) {
                return 5;
              }
            }
          }
          iVar2 = 0;
        }
      }
      else {
        iVar2 = 4;
      }
    }
    else {
      iVar2 = 4;
    }
  }
  return iVar2;
}
