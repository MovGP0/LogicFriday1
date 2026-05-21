/* 0043b10a FUN_0043b10a */

/* WARNING: Function: __security_check_cookie replaced with injection: security_check_cookie */

undefined4 __thiscall FUN_0043b10a(void *this,undefined4 param_1,undefined4 param_2)

{
  undefined4 *puVar1;
  void *pvVar2;
  undefined4 uVar3;
  uint unaff_retaddr;
  char local_2c [32];
  uint local_c;
  int local_8;
  
  local_c = DAT_00451a00 ^ unaff_retaddr;
  *(int *)((int)this + 0x28) = *(int *)((int)this + 0x28) + 1;
  pvVar2 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
  *(void **)((int)this + 0x2c) = pvVar2;
  if (*(int *)((int)this + 0x2c) == 0) {
    uVar3 = 0;
  }
  else {
    for (local_8 = *(int *)((int)this + 0x28) + -2; -1 < local_8; local_8 = local_8 + -1) {
      _memcpy((void *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c)),
              (void *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)),0x14);
    }
    puVar1 = *(undefined4 **)((int)this + 0x2c);
    *puVar1 = param_1;
    puVar1[1] = param_2;
    *(undefined4 *)(*(int *)((int)this + 0x2c) + 8) = 0;
    *(undefined4 *)(*(int *)((int)this + 0x2c) + 0x10) = *(undefined4 *)((int)this + 0x48);
    *(int *)((int)this + 0x48) = *(int *)((int)this + 0x48) + 1;
    if (**(int **)((int)this + 0x2c) == *(int *)(*(int *)((int)this + 0x2c) + 0x14)) {
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 0xc) = 0;
    }
    else {
      *(undefined4 *)(*(int *)((int)this + 0x2c) + 0xc) = 1;
    }
    if (DAT_00452ef4 != 0) {
      FUN_0043ed39(local_2c,(byte *)"iNodeCnt = %d");
      FUN_0040bdc3((LPARAM)local_2c);
    }
    uVar3 = 1;
  }
  return uVar3;
}
