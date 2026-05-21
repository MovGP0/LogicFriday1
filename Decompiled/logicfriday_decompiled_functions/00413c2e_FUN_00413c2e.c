/* 00413c2e FUN_00413c2e */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

void __thiscall FUN_00413c2e(void *this,FILE *param_1,int param_2,int *param_3)

{
  uint unaff_retaddr;
  uint local_174 [90];
  uint local_c;
  uint local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  if (*param_3 == 0x423) {
    FUN_0043ebd0(local_174,(uint *)&DAT_0044c1d4);
    FID_conflict__fwprintf(param_1,(wchar_t *)"int TestLUFcnZ( int iRow, int iOut )\n{\n");
    FUN_00413e13(this,param_1,local_174,*(uint *)(param_2 + 200));
  }
  else {
    FID_conflict__fwprintf(param_1,(wchar_t *)"int TestLUFcnZ( int iRow, int iOut )\n{\n\tint ");
    for (local_8 = 0; local_8 < *(uint *)(param_2 + 0xc4); local_8 = local_8 + 1) {
      if (local_8 == *(int *)(param_2 + 0xc4) - 1U) {
        FID_conflict__fwprintf(param_1,(wchar_t *)"%s=0;\n\n",param_2 + 0x160 + local_8 * 9);
      }
      else {
        FID_conflict__fwprintf(param_1,(wchar_t *)"%s=0, ",param_2 + 0x160 + local_8 * 9);
      }
    }
    for (local_8 = 0; local_8 < *(uint *)(param_2 + 0xc4); local_8 = local_8 + 1) {
      FID_conflict__fwprintf
                (param_1,(wchar_t *)"\tif( iRow & 1<<%d ) %s = 1;\n",(param_3[3] + -1) - local_8,
                 param_2 + 0x160 + local_8 * 9);
    }
    FUN_0043ebd0(local_174,(uint *)&DAT_0044ad26);
    for (local_8 = 0; local_8 < *(uint *)(param_2 + 0xc4); local_8 = local_8 + 1) {
      FUN_0043ebe0(local_174,(uint *)(param_2 + 0x160 + local_8 * 9));
      if (local_8 == *(int *)(param_2 + 0xc4) - 1U) {
        FUN_0043ebe0(local_174,(uint *)&DAT_0044ad26);
      }
      else {
        FUN_0043ebe0(local_174,(uint *)&DAT_0044c148);
      }
    }
    FUN_00413e13(this,param_1,local_174,*(uint *)(param_2 + 200));
  }
  return;
}
