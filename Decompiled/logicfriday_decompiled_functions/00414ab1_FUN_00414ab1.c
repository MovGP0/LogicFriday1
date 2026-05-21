/* 00414ab1 FUN_00414ab1 */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_00414ab1(void *this,int param_1,int param_2)

{
  uint uVar1;
  int iVar2;
  undefined4 uVar3;
  uint unaff_retaddr;
  uint local_430;
  wchar_t local_42c [514];
  uint local_28;
  uint local_24;
  int local_20;
  uint local_1c;
  uint local_18;
  FILE *local_14;
  uint local_10;
  uint local_c;
  uint local_8;
  
  local_28 = DAT_00451a00 ^ unaff_retaddr;
  local_14 = (FILE *)FUN_0043e6f2((char *)((int)this + 0x784),"w+t");
  if (local_14 == (FILE *)0x0) {
    GetLastError();
    uVar3 = 0x2f000d;
  }
  else {
    uVar1 = *(uint *)(*(int *)(param_1 + 4) + 0xc4);
    local_c = *(uint *)(*(int *)(param_1 + 4) + 200);
    FID_conflict__fwprintf(local_14,(wchar_t *)".i %d\n",uVar1);
    FID_conflict__fwprintf(local_14,(wchar_t *)".o %d\n",local_c);
    FID_conflict__fwprintf(local_14,L"椮扬");
    for (local_18 = 0; local_18 < uVar1; local_18 = local_18 + 1) {
      FID_conflict__fwprintf(local_14,L"┠s攮",*(int *)(param_1 + 4) + 0x160 + local_18 * 9);
    }
    FID_conflict__fwprintf(local_14,L"⸊扯");
    for (local_18 = 0; local_18 < local_c; local_18 = local_18 + 1) {
      FID_conflict__fwprintf(local_14,L"┠s攮",*(int *)(param_1 + 4) + 0xd0 + local_18 * 9);
    }
    FID_conflict__fwprintf(local_14,(wchar_t *)"\n.type fdr\n");
    if ((param_2 == 0) || (*(int *)(*(int *)(param_1 + 4) + 0x23c) == 0)) {
      local_1c = **(uint **)(param_1 + 4);
      for (local_18 = 0; local_18 < local_1c; local_18 = local_18 + 1) {
        FUN_0043ebd0((uint *)local_42c,(uint *)&DAT_0044ad26);
        local_430 = uVar1;
        while (local_430 = local_430 - 1, -1 < (int)local_430) {
          if ((local_18 & 1 << ((byte)local_430 & 0x1f)) == 0) {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb0);
          }
          else {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb4);
          }
        }
        FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044a7a4);
        for (local_20 = 0; local_20 < (int)local_c; local_20 = local_20 + 1) {
          iVar2 = *(int *)(*(int *)(*(int *)(param_1 + 4) + 0x84 + local_20 * 4) + local_18 * 4);
          if (iVar2 == 0) {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb0);
          }
          else if (iVar2 == 1) {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb4);
          }
          else {
            if (iVar2 != 2) {
              return 0x1d0002;
            }
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044ac88);
          }
        }
        FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044b734);
        FID_conflict__fwprintf(local_14,local_42c);
      }
    }
    else {
      local_1c = *(uint *)(*(int *)(param_1 + 4) + 500);
      for (local_18 = 0; local_18 < local_1c; local_18 = local_18 + 1) {
        FUN_0043ebd0((uint *)local_42c,(uint *)&DAT_0044ad26);
        local_8 = *(uint *)(*(int *)(*(int *)(param_1 + 4) + 0x1f8) + 8 + local_18 * 0xc);
        local_24 = *(uint *)(*(int *)(*(int *)(param_1 + 4) + 0x1f8) + 4 + local_18 * 0xc);
        local_430 = uVar1;
        while (local_430 = local_430 - 1, -1 < (int)local_430) {
          local_10 = 1 << ((byte)local_430 & 0x1f);
          if ((local_8 & local_10) == 0) {
            if ((local_24 & local_10) == 0) {
              FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb0);
            }
            else {
              FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb4);
            }
          }
          else {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044ac88);
          }
        }
        FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044a7a4);
        for (local_430 = 0; (int)local_430 < (int)local_c; local_430 = local_430 + 1) {
          if (*(int *)(*(int *)(*(int *)(param_1 + 4) + 0x1fc + local_430 * 4) + local_18 * 4) == 0)
          {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb0);
          }
          else {
            FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044bbb4);
          }
        }
        FUN_0043ebe0((uint *)local_42c,(uint *)&DAT_0044b734);
        FID_conflict__fwprintf(local_14,local_42c);
      }
    }
    FID_conflict__fwprintf(local_14,L"攮\n⸊祴数映牤\n⸊扯");
    _fclose(local_14);
    uVar3 = 0;
  }
  return uVar3;
}
