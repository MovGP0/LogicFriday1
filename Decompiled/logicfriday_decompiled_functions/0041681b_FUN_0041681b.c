/* 0041681b FUN_0041681b */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0041681b(void *this,undefined4 *param_1)

{
  void *pvVar1;
  uint unaff_retaddr;
  uint local_18 [3];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  for (local_8 = 0; local_8 < (int)param_1[0x32]; local_8 = local_8 + 1) {
    if (param_1[local_8 + 0x21] != 0) {
      _free((void *)param_1[local_8 + 0x21]);
    }
    param_1[local_8 + 0x21] = 0;
  }
  for (local_8 = 0; local_8 < 2; local_8 = local_8 + 1) {
    pvVar1 = _malloc(**(int **)((int)this + 8) << 2);
    param_1[local_8 + 0x21] = pvVar1;
  }
  pvVar1 = _malloc(**(int **)((int)this + 8) << 2);
  param_1[0x22] = pvVar1;
  _memcpy((void *)param_1[0x21],*(void **)(*(int *)((int)this + 8) + 0x84),
          **(int **)((int)this + 8) << 2);
  _memcpy((void *)param_1[0x22],*(void **)((int)this + 0x94),**(int **)((int)this + 0xc) << 2);
  *param_1 = **(undefined4 **)((int)this + 8);
  param_1[1] = *(undefined4 *)(*(int *)((int)this + 8) + 4);
  param_1[2] = *(undefined4 *)(*(int *)((int)this + 0xc) + 4);
  param_1[0x11] = *(undefined4 *)(*(int *)((int)this + 8) + 0x44);
  param_1[0x12] = *(undefined4 *)(*(int *)((int)this + 0xc) + 0x44);
  _memcpy(param_1 + 0x31,(void *)(*(int *)((int)this + 8) + 0xc4),300);
  if (*(int *)((int)this + 0x218) != 0) {
    for (local_8 = 0; local_8 < *(int *)((int)this + 0xd4); local_8 = local_8 + 1) {
      FUN_0043ed39((char *)local_18,&DAT_0044c754);
      FUN_0043ebd0((uint *)((int)param_1 + local_8 * 9 + 0x160),local_18);
    }
    param_1[0x33] = 1;
  }
  param_1[0x32] = 2;
  FUN_0043ebd0((uint *)((int)param_1 + 0xd9),(uint *)(*(int *)((int)this + 0xc) + 0xd0));
  return 0;
}
