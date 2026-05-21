/* 0043b061 FUN_0043b061 */

undefined4 __thiscall FUN_0043b061(void *this,int param_1)

{
  undefined4 uVar1;
  void *pvVar2;
  undefined4 local_8;
  
  if ((param_1 < 0) || (*(int *)((int)this + 0x30) + -1 < param_1)) {
    uVar1 = 0;
  }
  else {
    if ((1 < *(int *)((int)this + 0x30)) && (param_1 < *(int *)((int)this + 0x30) + -1)) {
      for (local_8 = param_1; local_8 < *(int *)((int)this + 0x30) + -1; local_8 = local_8 + 1) {
        _memcpy((void *)(local_8 * 0x14 + *(int *)((int)this + 0x34)),
                (void *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x34)),0x14);
      }
    }
    *(int *)((int)this + 0x30) = *(int *)((int)this + 0x30) + -1;
    pvVar2 = _realloc(*(void **)((int)this + 0x34),*(int *)((int)this + 0x30) * 0x14);
    *(void **)((int)this + 0x34) = pvVar2;
    uVar1 = 1;
  }
  return uVar1;
}
