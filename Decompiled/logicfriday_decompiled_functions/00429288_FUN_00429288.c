/* 00429288 FUN_00429288 */

/* WARNING: Function: __chkstk replaced with injection: alloca_probe */
/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

int __thiscall FUN_00429288(void *this,int param_1,int param_2,int param_3,int param_4)

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
  
  local_8 = 0x429295;
  local_28 = DAT_00451a00 ^ unaff_retaddr;
  if (param_3 < param_2) {
    local_c = param_3;
    local_14 = param_2;
  }
  else {
    local_c = param_2;
    local_14 = param_3;
  }
  local_20 = 0;
  for (local_10 = 0;
      local_10 < *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 0x14);
      local_10 = local_10 + 1) {
    local_18 = local_10 / 0x20;
    local_8 = 1 << ((byte)(local_10 % 0x20) & 0x1f);
    local_20 = 1;
    for (local_24 = local_c; local_24 <= local_14; local_24 = local_24 + 1) {
      if ((*(uint *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + local_24 * 0x48 + 0x28 +
                    local_18 * 4) & local_8) != 0) {
        local_20 = 0;
        break;
      }
    }
    if (local_20 != 0) break;
  }
  if (((DAT_00452ef4 != 0) && (local_20 == 0)) &&
     ((*(int *)((int)this + 0x2680) != 0 || (param_4 != 0)))) {
    FUN_0043ed39((char *)local_202c,
                 (byte *)
                 "Out of horz slots.\nbFirstPass: %d\nbAlloc: %d  Col1: %d  Col2: %d  Row: %d  SlotCntH: %d  iX: %d\ndwSlotMask[iX]:\n"
                );
    for (local_10 = local_c; local_10 <= local_14; local_10 = local_10 + 1) {
      FUN_0043ed39((char *)local_206c,&DAT_0044d394);
      FUN_0043ebe0(local_202c,local_206c);
    }
    MessageBoxA(*(HWND *)((int)this + 0x16f0),(LPCSTR)local_202c,"",0);
  }
  if ((param_4 != 0) && (local_20 != 0)) {
    for (local_1c = local_c; local_1c <= local_14; local_1c = local_1c + 1) {
      *(uint *)(local_1c * 0x48 + *(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 0x28 +
               local_18 * 4) =
           *(uint *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + local_1c * 0x48 + 0x28 +
                    local_18 * 4) | local_8;
      if (*(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 0x18 + local_1c * 0x48) <
          local_10 + 1) {
        *(int *)(*(int *)(*(int *)((int)this + 0x2678) + param_1 * 4) + 0x18 + local_1c * 0x48) =
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
