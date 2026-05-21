/* 00428eb1 FUN_00428eb1 */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall FUN_00428eb1(void *this,int param_1,int param_2,int param_3,int param_4,int param_5)

{
  uint unaff_retaddr;
  uint local_206c [16];
  uint local_202c [2049];
  uint local_28;
  int local_24;
  int local_20;
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  uint local_8;
  
  local_8 = 0x428ebe;
  local_28 = DAT_00451a00 ^ unaff_retaddr;
  local_20 = 0;
  if (param_2 < param_1) {
    local_c = param_2;
    local_14 = param_1;
  }
  else {
    local_c = param_1;
    local_14 = param_2;
  }
  if (param_4 == 0) {
    local_10 = *(int *)(**(int **)((int)this + 0x2678) + 8 + param_3 * 0x48);
    do {
      local_10 = local_10 + -1;
      if (local_10 < 0) break;
      local_18 = local_10 / 0x20;
      local_8 = 1 << ((byte)(local_10 % 0x20) & 0x1f);
      local_20 = 1;
      for (local_24 = local_c; local_24 <= local_14; local_24 = local_24 + 1) {
        if ((*(uint *)(*(int *)(*(int *)((int)this + 0x2678) + local_24 * 4) + param_3 * 0x48 + 0x38
                      + local_18 * 4) & local_8) != 0) {
          local_20 = 0;
          break;
        }
      }
    } while (local_20 == 0);
  }
  else {
    for (local_10 = 0; local_10 < *(int *)(**(int **)((int)this + 0x2678) + 8 + param_3 * 0x48);
        local_10 = local_10 + 1) {
      local_18 = local_10 / 0x20;
      local_8 = 1 << ((byte)(local_10 % 0x20) & 0x1f);
      local_20 = 1;
      for (local_24 = local_c; local_24 <= local_14; local_24 = local_24 + 1) {
        if ((*(uint *)(*(int *)(*(int *)((int)this + 0x2678) + local_24 * 4) + param_3 * 0x48 + 0x38
                      + local_18 * 4) & local_8) != 0) {
          local_20 = 0;
          break;
        }
      }
      if (local_20 != 0) break;
    }
  }
  if (((DAT_00452ef4 != 0) && (local_20 == 0)) && (param_5 != 0)) {
    FUN_0043ed39((char *)local_202c,
                 (byte *)
                 "Out of vert slots.\nbFirstPass: %d bAlloc: %d  Row1: %d  Row2: %d  Col: %d  SlotCntV: %d  iX: %d\ndwSlotMask[iX]:\n"
                );
    for (local_10 = local_c; local_10 <= local_14; local_10 = local_10 + 1) {
      FUN_0043ed39((char *)local_206c,&DAT_0044d394);
      FUN_0043ebe0(local_202c,local_206c);
    }
    MessageBoxA(*(HWND *)((int)this + 0x16f0),(LPCSTR)local_202c,"",0);
  }
  if ((param_5 != 0) && (local_20 != 0)) {
    for (local_1c = local_c; local_1c <= local_14; local_1c = local_1c + 1) {
      *(uint *)(param_3 * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + 0x38 +
               local_18 * 4) =
           *(uint *)(*(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + param_3 * 0x48 + 0x38 +
                    local_18 * 4) | local_8;
      if (param_4 == 0) {
        if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + 0x10 + param_3 * 0x48)
            < *(int *)(**(int **)((int)this + 0x2678) + 8 + param_3 * 0x48) - local_10) {
          *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + 0x10 + param_3 * 0x48) =
               *(int *)(**(int **)((int)this + 0x2678) + 8 + param_3 * 0x48) - local_10;
        }
      }
      else if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + 0xc + param_3 * 0x48
                       ) < local_10 + 1) {
        *(int *)(*(int *)(*(int *)((int)this + 0x2678) + local_1c * 4) + 0xc + param_3 * 0x48) =
             local_10 + 1;
      }
    }
  }
  if (local_20 == 0) {
    local_10 = 0;
  }
  else {
    local_10 = local_10 + 1;
  }
  return local_10;
}
