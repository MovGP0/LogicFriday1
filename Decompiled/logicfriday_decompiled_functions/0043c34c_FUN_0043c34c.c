/* 0043c34c FUN_0043c34c */

undefined4 __thiscall FUN_0043c34c(void *this,int *param_1)

{
  int iVar1;
  int iVar2;
  int local_c;
  
  local_c = 0;
  do {
    if (*(int *)((int)this + 0x28) + -1 <= local_c) {
      return 0;
    }
    iVar1 = local_c + 1;
    if (param_1[4] == 1) {
      if ((((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) !=
             *(int *)(iVar1 * 0x14 + *(int *)((int)this + 0x2c))) &&
           (iVar2 = FUN_0043f3b8(*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) -
                                 param_1[1]), iVar2 < 7)) &&
          ((*(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) <= *param_1 ||
           (*(int *)(iVar1 * 0x14 + *(int *)((int)this + 0x2c)) <= *param_1)))) &&
         ((*param_1 <= *(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c)) ||
          (*param_1 <= *(int *)(iVar1 * 0x14 + *(int *)((int)this + 0x2c)))))) {
        param_1[2] = *param_1;
        param_1[3] = *(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14);
        param_1[4] = 1;
        iVar2 = *(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14);
        param_1[7] = *(int *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
        param_1[8] = iVar2;
        iVar2 = *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14);
        param_1[9] = *(int *)(*(int *)((int)this + 0x2c) + iVar1 * 0x14);
        param_1[10] = iVar2;
        param_1[0xd] = 0;
        return 1;
      }
    }
    else if (((((param_1[4] == 0) &&
               (*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) !=
                *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14))) &&
              (iVar2 = FUN_0043f3b8(*(int *)(*(int *)((int)this + 0x2c) + local_c * 0x14) - *param_1
                                   ), iVar2 < 7)) &&
             ((*(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) <= param_1[1] ||
              (*(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14) <= param_1[1])))) &&
            ((param_1[1] <= *(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14) ||
             (param_1[1] <= *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14))))) {
      param_1[3] = param_1[1];
      param_1[2] = *(int *)(local_c * 0x14 + *(int *)((int)this + 0x2c));
      param_1[4] = 0;
      iVar2 = *(int *)(*(int *)((int)this + 0x2c) + 4 + local_c * 0x14);
      param_1[7] = *(int *)(*(int *)((int)this + 0x2c) + local_c * 0x14);
      param_1[8] = iVar2;
      iVar2 = *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar1 * 0x14);
      param_1[9] = *(int *)(*(int *)((int)this + 0x2c) + iVar1 * 0x14);
      param_1[10] = iVar2;
      param_1[0xd] = 0;
      return 1;
    }
    local_c = local_c + 1;
  } while( true );
}
