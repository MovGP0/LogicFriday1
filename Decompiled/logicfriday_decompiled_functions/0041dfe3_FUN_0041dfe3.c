/* 0041dfe3 FUN_0041dfe3 */

undefined4 __thiscall FUN_0041dfe3(void *this,uint *param_1,undefined4 *param_2,undefined4 *param_3)

{
  size_t sVar1;
  void *pvVar2;
  
  FUN_0043ed39(*(char **)((int)this + 0x268),&DAT_0044cba0);
  *(undefined4 *)((int)this + 0x23c) = 0;
  *(undefined4 *)((int)this + 0x244) = 0;
  *param_2 = this;
  sVar1 = _strlen((char *)param_1);
  if (*(int *)((int)this + 0x165c) * 0x7fff + -0x100 < (int)sVar1) {
    *(int *)((int)this + 0x165c) = (int)sVar1 / 0x7fff + 1;
    pvVar2 = _realloc(*(void **)((int)this + 0x268),*(int *)((int)this + 0x165c) * 0x7fff);
    *(void **)((int)this + 0x268) = pvVar2;
  }
  FUN_0043ebe0(*(uint **)((int)this + 0x268),param_1);
  *param_3 = *(undefined4 *)((int)this + 0x268);
  FUN_004219f6(this,*(uint **)((int)this + 0x268));
  return 0;
}
