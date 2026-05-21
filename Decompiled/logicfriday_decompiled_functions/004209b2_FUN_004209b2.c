/* 004209b2 FUN_004209b2 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_004209b2(void *this,int param_1,char *param_2)

{
  uint uVar1;
  size_t sVar2;
  void *pvVar3;
  undefined1 *puVar4;
  uint unaff_retaddr;
  int local_6c;
  int local_64;
  int local_5c;
  int local_58;
  uint local_54 [8];
  uint local_34;
  int local_30;
  uint local_2c;
  int local_28;
  int local_24;
  int local_20;
  uint local_1c;
  int local_18;
  uint local_14;
  undefined4 local_10;
  undefined1 *local_c;
  int local_8;
  
  local_34 = DAT_00451a00 ^ unaff_retaddr;
  local_20 = *(int *)((int)this + 0xc4) + -1;
  local_30 = 0;
  local_28 = 0;
  local_10 = 0;
  local_c = (undefined1 *)0x0;
  local_18 = 0;
  local_64 = 0;
  local_8 = 0;
  FUN_0043ebd0(local_54,(uint *)&DAT_0044ad26);
  if (param_1 == 0) {
    if (*(int *)((int)this + 0x240) == 0) {
      FUN_0043ed39(*(char **)((int)this + 0x268),&DAT_0044cba0);
    }
    else {
      FUN_0043ed39(*(char **)((int)this + 0x268),&DAT_0044cba0);
    }
  }
  else {
    sVar2 = _strlen(param_2);
    if (sVar2 == 0) {
      FUN_0043ebd0(*(uint **)((int)this + 0x268),(uint *)&DAT_0044ad26);
    }
    else {
      FUN_0043ed39(*(char **)((int)this + 0x268),&DAT_0044cba0);
    }
  }
  for (local_24 = 0; local_24 < *(int *)((int)this + 0xc4); local_24 = local_24 + 1) {
    sVar2 = _strlen((char *)((int)this + local_24 * 9 + 0x160));
    local_64 = local_64 + sVar2;
  }
  local_18 = local_64 + 2 + *(int *)((int)this + 0xc4) * 2;
  sVar2 = _strlen(*(char **)((int)this + 0x268));
  local_58 = sVar2 + 10;
  if ((*(int *)((int)this + 0x23c) == 0) || (*(int *)((int)this + 0x240) == 0)) {
    for (local_24 = 0; local_24 < *(int *)((int)this + 200); local_24 = local_24 + 1) {
      local_58 = local_58 + *(int *)((int)this + local_24 * 4 + 4) * local_18;
    }
  }
  else {
    for (local_24 = 0; local_24 < *(int *)((int)this + 200); local_24 = local_24 + 1) {
      for (local_6c = 0; local_6c < *(int *)((int)this + 500); local_6c = local_6c + 1) {
        if (*(int *)(*(int *)((int)this + local_24 * 4 + 0x1fc) + local_6c * 4) == 1) {
          local_8 = local_8 + 1;
        }
      }
    }
    local_58 = local_58 + local_8 * local_18;
  }
  if (*(int *)((int)this + 0x165c) * 0x7fff + -0xfa < local_58) {
    *(int *)((int)this + 0x165c) = local_58 / 0x7fff + 1;
    pvVar3 = _realloc(*(void **)((int)this + 0x268),*(int *)((int)this + 0x165c) * 0x7fff);
    *(void **)((int)this + 0x268) = pvVar3;
  }
  *(undefined4 *)((int)this + 0x250) = 1;
  for (local_24 = 0; local_24 < *(int *)((int)this + 200); local_24 = local_24 + 1) {
    if ((*(int *)((int)this + local_24 * 4 + 4) != *(int *)this) &&
       (*(int *)((int)this + local_24 * 4 + 4) != 0)) {
      *(undefined4 *)((int)this + 0x250) = 0;
    }
  }
  *(undefined4 *)((int)this + 0x244) = 0;
  sVar2 = _strlen(*(char **)((int)this + 0x268));
  local_c = (undefined1 *)(sVar2 + *(int *)((int)this + 0x268));
  for (local_5c = 0; local_5c < *(int *)((int)this + 200); local_5c = local_5c + 1) {
    local_10 = 0;
    local_28 = 0;
    local_30 = 0;
    FUN_0043ed39((char *)local_54,(byte *)"%s = ");
    local_24 = 0;
    while (sVar2 = _strlen((char *)local_54), local_24 < (int)sVar2) {
      *local_c = *(undefined1 *)((int)local_54 + local_24);
      local_c = local_c + 1;
      local_24 = local_24 + 1;
    }
    if (*(int *)((int)this + local_5c * 4 + 4) + *(int *)((int)this + local_5c * 4 + 0x44) ==
        *(int *)this) {
      *local_c = 0x31;
      local_c[1] = 0x3b;
      local_c[2] = 10;
      local_c = local_c + 3;
    }
    else if (*(int *)((int)this + local_5c * 4 + 4) == 0) {
      *local_c = 0x30;
      local_c[1] = 0x3b;
      local_c[2] = 10;
      local_c = local_c + 3;
    }
    else {
      if ((*(int *)((int)this + 0x23c) == 0) || (*(int *)((int)this + 0x240) == 0)) {
        *(undefined4 *)((int)this + 0x244) = 0;
        local_1c = *(uint *)this;
        for (local_2c = 0; local_2c < local_1c; local_2c = local_2c + 1) {
          if (*(int *)(*(int *)((int)this + local_5c * 4 + 0x84) + local_2c * 4) == 1) {
            if (local_28 != 0) {
              *local_c = 0x20;
              local_c[1] = 0x2b;
              local_c[2] = 0x20;
              local_c = local_c + 3;
            }
            local_28 = 1;
            for (local_6c = local_20; -1 < local_6c; local_6c = local_6c + -1) {
              sVar2 = _strlen((char *)((int)this + (local_20 - local_6c) * 9 + 0x160));
              for (local_24 = 0; local_24 < (int)sVar2; local_24 = local_24 + 1) {
                *local_c = *(undefined1 *)((int)this + local_24 + (local_20 - local_6c) * 9 + 0x160)
                ;
                local_c = local_c + 1;
              }
              if ((local_2c & 1 << ((byte)local_6c & 0x1f)) == 0) {
                *local_c = 0x27;
                local_c = local_c + 1;
              }
              if (local_6c != 0) {
                *local_c = 0x20;
                local_c = local_c + 1;
              }
            }
          }
        }
        *local_c = 0x3b;
        puVar4 = local_c;
      }
      else {
        *(undefined4 *)((int)this + 0x244) = 1;
        for (local_2c = 0; local_2c < *(uint *)((int)this + 500); local_2c = local_2c + 1) {
          if (*(int *)(*(int *)((int)this + local_5c * 4 + 0x1fc) + local_2c * 4) != 0) {
            if (local_30 != 0) {
              *local_c = 0x20;
              local_c[1] = 0x2b;
              local_c[2] = 0x20;
              local_c = local_c + 3;
            }
            local_30 = 1;
            uVar1 = *(uint *)(*(int *)((int)this + 0x1f8) + 4 + local_2c * 0xc);
            local_14 = *(uint *)(*(int *)((int)this + 0x1f8) + 8 + local_2c * 0xc);
            for (local_6c = local_20; -1 < local_6c; local_6c = local_6c + -1) {
              if ((local_14 & 1 << ((byte)local_6c & 0x1f)) == 0) {
                sVar2 = _strlen((char *)((int)this + (local_20 - local_6c) * 9 + 0x160));
                for (local_24 = 0; local_24 < (int)sVar2; local_24 = local_24 + 1) {
                  *local_c = *(undefined1 *)
                              ((int)this + local_24 + (local_20 - local_6c) * 9 + 0x160);
                  local_c = local_c + 1;
                }
                if ((uVar1 & 1 << ((byte)local_6c & 0x1f)) == 0) {
                  *local_c = 0x27;
                  local_c = local_c + 1;
                }
                if (local_6c != 0) {
                  *local_c = 0x20;
                  local_c = local_c + 1;
                }
              }
            }
          }
        }
        *local_c = 0x3b;
        puVar4 = local_c;
      }
      local_c = puVar4 + 1;
      *local_c = 10;
      local_c = puVar4 + 2;
    }
  }
  *local_c = 10;
  puVar4 = local_c + 1;
  if (puVar4 != (undefined1 *)0x0) {
    *puVar4 = 0;
    puVar4 = local_c + 2;
  }
  local_c = puVar4;
  FUN_004219f6(this,*(uint **)((int)this + 0x268));
  return 0;
}
