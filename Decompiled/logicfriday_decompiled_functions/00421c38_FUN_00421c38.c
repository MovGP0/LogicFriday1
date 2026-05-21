/* 00421c38 FUN_00421c38 */

undefined4 __thiscall FUN_00421c38(void *this,int param_1)

{
  void *pvVar1;
  undefined4 uVar2;
  uint local_8;
  
  *(int *)((int)this + 500) = param_1;
  pvVar1 = _realloc(*(void **)((int)this + 0x1f8),param_1 * 0xc);
  *(void **)((int)this + 0x1f8) = pvVar1;
  if (*(int *)((int)this + 0x1f8) == 0) {
    uVar2 = 0x40014;
  }
  else {
    _memset(*(void **)((int)this + 0x1f8),0,param_1 * 0xc);
    *(undefined4 *)((int)this + 0x23c) = 1;
    for (local_8 = 0; local_8 < *(uint *)((int)this + 200); local_8 = local_8 + 1) {
      pvVar1 = _realloc(*(void **)((int)this + local_8 * 4 + 0x1fc),param_1 << 2);
      *(void **)((int)this + local_8 * 4 + 0x1fc) = pvVar1;
      if (*(int *)((int)this + local_8 * 4 + 0x1fc) == 0) {
        return 0x40015;
      }
      _memset(*(void **)((int)this + local_8 * 4 + 0x1fc),0,param_1 << 2);
    }
    uVar2 = 0;
  }
  return uVar2;
}
