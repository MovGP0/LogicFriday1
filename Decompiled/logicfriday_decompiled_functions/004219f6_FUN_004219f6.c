/* 004219f6 FUN_004219f6 */

void __thiscall FUN_004219f6(void *this,uint *param_1)

{
  size_t sVar1;
  size_t sVar2;
  void *pvVar3;
  
  sVar1 = _strlen(*(char **)((int)this + 0x26c));
  sVar2 = _strlen((char *)param_1);
  if (*(int *)((int)this + 0x1660) * 0x7fff - 0x100U < sVar1 + sVar2) {
    sVar1 = _strlen(*(char **)((int)this + 0x26c));
    sVar2 = _strlen((char *)param_1);
    *(uint *)((int)this + 0x1660) = (sVar2 + sVar1) / 0x7fff + 1;
    pvVar3 = _realloc(*(void **)((int)this + 0x26c),*(int *)((int)this + 0x1660) * 0x7fff);
    *(void **)((int)this + 0x26c) = pvVar3;
  }
  FUN_0043ebe0(*(uint **)((int)this + 0x26c),param_1);
  return;
}
