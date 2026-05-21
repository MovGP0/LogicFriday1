/* 0042166a FUN_0042166a */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0042166a(void *this,int param_1)

{
  uint uVar1;
  bool bVar2;
  size_t sVar3;
  void *pvVar4;
  uint unaff_retaddr;
  int local_50;
  int local_48;
  int local_40;
  char local_34 [16];
  uint local_24;
  int local_20;
  int local_1c;
  int local_18;
  int local_14;
  uint local_10;
  int local_c;
  char *local_8;
  
  local_24 = DAT_00451a00 ^ unaff_retaddr;
  local_1c = *(int *)((int)this + 0xc4) + -1;
  local_14 = 0;
  local_48 = 0;
  local_c = 0;
  for (local_20 = 0; local_20 < *(int *)((int)this + 0xc4); local_20 = local_20 + 1) {
    sVar3 = _strlen((char *)((int)this + local_20 * 9 + 0x160));
    local_48 = local_48 + sVar3;
  }
  local_14 = local_48 + 1 + *(int *)((int)this + 0xc4) * 2;
  for (local_20 = 0; local_20 < *(int *)((int)this + 200); local_20 = local_20 + 1) {
    for (local_50 = 0; local_50 < *(int *)(param_1 + 4); local_50 = local_50 + 1) {
      if (*(int *)(*(int *)(param_1 + 0xc + local_20 * 4) + local_50 * 4) == 1) {
        local_c = local_c + 1;
      }
    }
  }
  pvVar4 = _malloc(local_c * local_14 + *(int *)((int)this + 200) * 0xb + 0x100);
  *(void **)((int)this + 0x270) = pvVar4;
  FUN_0043ebd0(*(uint **)((int)this + 0x270),(uint *)&DAT_0044ad26);
  local_8 = *(char **)((int)this + 0x270);
  for (local_40 = 0; local_40 < *(int *)((int)this + 200); local_40 = local_40 + 1) {
    FUN_0043ed39(local_34,(byte *)"%s = ");
    local_20 = 0;
    while (sVar3 = _strlen(local_34), local_20 < (int)sVar3) {
      *local_8 = local_34[local_20];
      local_8 = local_8 + 1;
      local_20 = local_20 + 1;
    }
    if (*(int *)this == *(int *)((int)this + local_40 * 4 + 4)) {
      *local_8 = '1';
      local_8[1] = ';';
      local_8[2] = '\n';
      local_8 = local_8 + 3;
    }
    else if (*(int *)((int)this + local_40 * 4 + 4) == 0) {
      *local_8 = '0';
      local_8[1] = ';';
      local_8[2] = '\n';
      local_8 = local_8 + 3;
    }
    else {
      local_20 = *(int *)(param_1 + 4);
      while (local_20 = local_20 + -1, -1 < local_20) {
        if (*(int *)(*(int *)(param_1 + 0xc + local_40 * 4) + local_20 * 4) != 0) {
          uVar1 = *(uint *)(*(int *)(param_1 + 8) + 4 + local_20 * 0xc);
          local_10 = *(uint *)(*(int *)(param_1 + 8) + 8 + local_20 * 0xc);
          *local_8 = '(';
          local_8 = local_8 + 1;
          bVar2 = false;
          for (local_50 = local_1c; -1 < local_50; local_50 = local_50 + -1) {
            if ((local_10 & 1 << ((byte)local_50 & 0x1f)) == 0) {
              if (bVar2) {
                *local_8 = '+';
                local_8 = local_8 + 1;
              }
              sVar3 = _strlen((char *)((int)this + (local_1c - local_50) * 9 + 0x160));
              for (local_18 = 0; local_18 < (int)sVar3; local_18 = local_18 + 1) {
                *local_8 = *(char *)((int)this + local_18 + (local_1c - local_50) * 9 + 0x160);
                local_8 = local_8 + 1;
              }
              bVar2 = true;
              if ((uVar1 & 1 << ((byte)local_50 & 0x1f)) != 0) {
                *local_8 = '\'';
                local_8 = local_8 + 1;
              }
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
  *local_8 = '\0';
  *(undefined4 *)((int)this + 0x24c) = 1;
  return 0;
}
