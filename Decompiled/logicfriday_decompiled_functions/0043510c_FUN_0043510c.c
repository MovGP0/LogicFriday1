/* 0043510c FUN_0043510c */

undefined4 __thiscall FUN_0043510c(void *this,int param_1)

{
  int iVar1;
  bool bVar2;
  int local_2c;
  int local_24;
  int local_20;
  int local_18;
  int local_10;
  int local_c;
  int local_8;
  
  local_20 = 0;
  local_c = 1;
  if (*(uint *)((int)this + 0xc4) < *(uint *)((int)this + 200)) {
    local_2c = *(int *)((int)this + 200);
  }
  else {
    local_2c = *(int *)((int)this + 0xc4);
  }
  local_8 = local_2c;
  do {
    local_18 = 0;
    if (param_1 <= local_20) {
      *(int *)((int)this + 0x2670) = local_8;
      *(int *)((int)this + 0x2674) = local_c + 1;
      for (local_10 = 0; local_10 < *(int *)((int)this + 0x1650); local_10 = local_10 + 1) {
        if ((*(int *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) == 0) &&
           (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4)) == 9)) {
          *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) = local_c;
        }
      }
      **(undefined4 **)((int)this + 0x1668) = *(undefined4 *)((int)this + 0xc4);
      *(undefined4 *)(*(int *)((int)this + 0x1668) + local_c * 4) = *(undefined4 *)((int)this + 200)
      ;
      return 0;
    }
    for (local_10 = 0; local_10 < *(int *)((int)this + 0x1650); local_10 = local_10 + 1) {
      if ((((*(int *)(*(int *)((int)this + 0x3a4) + 0x48 + local_10 * 0xfc) == 0) &&
           (*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) == -3)) &&
          (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4)) != 8)) &&
         (*(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4)) != 9)) {
        bVar2 = false;
        for (local_24 = 0; local_24 < *(int *)(*(int *)((int)this + 0x3a4) + 0x18 + local_10 * 0xfc)
            ; local_24 = local_24 + 1) {
          iVar1 = *(int *)(local_10 * 0xfc + *(int *)((int)this + 0x3a4) + 0x1c + local_24 * 4);
          if (((iVar1 != -2) && (iVar1 != -1)) &&
             ((*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + iVar1 * 0xfc) == -3 ||
              (*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + iVar1 * 0xfc) == local_c)))) {
            bVar2 = true;
            break;
          }
        }
        if (!bVar2) {
          *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + local_10 * 0xfc) = local_c;
          local_18 = local_18 + 1;
          local_20 = local_20 + 1;
        }
      }
    }
    *(int *)(*(int *)((int)this + 0x1668) + local_c * 4) = local_18;
    if (local_8 < local_18) {
      local_8 = local_18;
    }
    local_c = local_c + 1;
  } while( true );
}
