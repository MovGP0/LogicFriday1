/* 0043c82b FUN_0043c82b */

undefined4 __thiscall FUN_0043c82b(void *this,int param_1)

{
  void *pvVar1;
  undefined4 local_8;
  
  for (local_8 = param_1; local_8 < *(int *)((int)this + 0x28) + -1; local_8 = local_8 + 1) {
    _memcpy((void *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)),
            (void *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c)),0x14);
  }
  *(int *)((int)this + 0x28) = *(int *)((int)this + 0x28) + -1;
  pvVar1 = _realloc(*(void **)((int)this + 0x2c),*(int *)((int)this + 0x28) * 0x14);
  *(void **)((int)this + 0x2c) = pvVar1;
  return 1;
}
