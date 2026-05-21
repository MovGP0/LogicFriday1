/* 0043af87 FUN_0043af87 */

bool __thiscall FUN_0043af87(void *this,int param_1)

{
  bool bVar1;
  void *pvVar2;
  int local_10;
  int local_8;
  
  bVar1 = false;
  local_8 = 0;
  do {
    if (*(int *)((int)this + 0x30) <= local_8) {
LAB_0043b026:
      if (bVar1) {
        *(int *)((int)this + 0x30) = *(int *)((int)this + 0x30) + -1;
        pvVar2 = _realloc(*(void **)((int)this + 0x34),*(int *)((int)this + 0x30) * 0x14);
        *(void **)((int)this + 0x34) = pvVar2;
      }
      return bVar1;
    }
    if (*(int *)(*(int *)((int)this + 0x34) + 0xc + local_8 * 0x14) == param_1) {
      bVar1 = true;
      if ((1 < *(int *)((int)this + 0x30)) && (local_8 < *(int *)((int)this + 0x30) + -1)) {
        for (local_10 = local_8; local_10 < *(int *)((int)this + 0x30) + -1; local_10 = local_10 + 1
            ) {
          _memcpy((void *)(local_10 * 0x14 + *(int *)((int)this + 0x34)),
                  (void *)((local_10 + 1) * 0x14 + *(int *)((int)this + 0x34)),0x14);
        }
      }
      goto LAB_0043b026;
    }
    local_8 = local_8 + 1;
  } while( true );
}
