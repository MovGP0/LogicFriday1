/* 0043bad3 FUN_0043bad3 */

undefined4 __thiscall FUN_0043bad3(void *this,POINT *param_1)

{
  LONG LVar1;
  int iVar2;
  int iVar3;
  BOOL BVar4;
  int local_1c;
  tagRECT local_14;
  
  local_1c = 0;
  do {
    if (*(int *)((int)this + 0x28) + -1 <= local_1c) {
      return 0;
    }
    iVar2 = local_1c + 1;
    if (*(int *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14) ==
        *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar2 * 0x14)) {
      iVar3 = FUN_0043f3b8(*(int *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14) - param_1->y);
      if (((iVar3 < 7) &&
          ((*(int *)(local_1c * 0x14 + *(int *)((int)this + 0x2c)) <= param_1->x ||
           (*(int *)(iVar2 * 0x14 + *(int *)((int)this + 0x2c)) <= param_1->x)))) &&
         ((param_1->x <= *(int *)(local_1c * 0x14 + *(int *)((int)this + 0x2c)) ||
          (param_1->x <= *(int *)(iVar2 * 0x14 + *(int *)((int)this + 0x2c)))))) {
        param_1[1].x = param_1->x;
        param_1[1].y = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14);
        param_1[2].x = 1;
        LVar1 = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14);
        param_1[3].y = *(LONG *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        param_1[4].x = LVar1;
        LVar1 = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + iVar2 * 0x14);
        param_1[4].y = *(LONG *)(*(int *)((int)this + 0x2c) + iVar2 * 0x14);
        param_1[5].x = LVar1;
        param_1[5].y = local_1c;
        param_1[6].x = iVar2;
        param_1[6].y = 0;
        if (*(int *)((int)this + 0x3c) != 0) {
          if (*(int *)this == -3) {
            local_14.left = **(LONG **)((int)this + 0x2c);
            local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4);
            local_14.right = local_14.left;
            local_14.bottom = local_14.top;
            InflateRect(&local_14,6,6);
            BVar4 = PtInRect(&local_14,*param_1);
            if (BVar4 != 0) {
              param_1[1].x = **(LONG **)((int)this + 0x2c);
              param_1[1].y = *(LONG *)(*(int *)((int)this + 0x2c) + 4);
              param_1[6].y = 1;
            }
          }
          else if (*(int *)((int)this + 0x14) == -3) {
            local_14.left =
                 *(LONG *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c));
            local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4 +
                                    (*(int *)((int)this + 0x28) + -1) * 0x14);
            local_14.right = local_14.left;
            local_14.bottom = local_14.top;
            InflateRect(&local_14,6,6);
            BVar4 = PtInRect(&local_14,*param_1);
            if (BVar4 != 0) {
              param_1[1].x = *(LONG *)((*(int *)((int)this + 0x28) + -1) * 0x14 +
                                      *(int *)((int)this + 0x2c));
              param_1[1].y = *(LONG *)(*(int *)((int)this + 0x2c) + 4 +
                                      (*(int *)((int)this + 0x28) + -1) * 0x14);
              param_1[6].y = 1;
            }
          }
        }
        return 1;
      }
    }
    else {
      iVar3 = FUN_0043f3b8(*(int *)(*(int *)((int)this + 0x2c) + local_1c * 0x14) - param_1->x);
      if (((iVar3 < 7) &&
          ((*(int *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14) <= param_1->y ||
           (*(int *)(*(int *)((int)this + 0x2c) + 4 + iVar2 * 0x14) <= param_1->y)))) &&
         ((param_1->y <= *(int *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14) ||
          (param_1->y <= *(int *)(*(int *)((int)this + 0x2c) + 4 + iVar2 * 0x14))))) {
        param_1[1].y = param_1->y;
        param_1[1].x = *(LONG *)(local_1c * 0x14 + *(int *)((int)this + 0x2c));
        param_1[2].x = 0;
        LVar1 = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + local_1c * 0x14);
        param_1[3].y = *(LONG *)(*(int *)((int)this + 0x2c) + local_1c * 0x14);
        param_1[4].x = LVar1;
        LVar1 = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + iVar2 * 0x14);
        param_1[4].y = *(LONG *)(*(int *)((int)this + 0x2c) + iVar2 * 0x14);
        param_1[5].x = LVar1;
        param_1[5].y = local_1c;
        param_1[6].x = iVar2;
        param_1[6].y = 0;
        if (*(int *)((int)this + 0x3c) != 0) {
          if (*(int *)this == -3) {
            local_14.left = **(LONG **)((int)this + 0x2c);
            local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4);
            local_14.right = local_14.left;
            local_14.bottom = local_14.top;
            InflateRect(&local_14,6,6);
            BVar4 = PtInRect(&local_14,*param_1);
            if (BVar4 != 0) {
              param_1[1].x = **(LONG **)((int)this + 0x2c);
              param_1[1].y = *(LONG *)(*(int *)((int)this + 0x2c) + 4);
              param_1[6].y = 1;
            }
          }
          else if (*(int *)((int)this + 0x14) == -3) {
            local_14.left =
                 *(LONG *)((*(int *)((int)this + 0x28) + -1) * 0x14 + *(int *)((int)this + 0x2c));
            local_14.top = *(LONG *)(*(int *)((int)this + 0x2c) + 4 +
                                    (*(int *)((int)this + 0x28) + -1) * 0x14);
            local_14.right = local_14.left;
            local_14.bottom = local_14.top;
            InflateRect(&local_14,6,6);
            BVar4 = PtInRect(&local_14,*param_1);
            if (BVar4 != 0) {
              param_1[1].x = *(LONG *)((*(int *)((int)this + 0x28) + -1) * 0x14 +
                                      *(int *)((int)this + 0x2c));
              param_1[1].y = *(LONG *)(*(int *)((int)this + 0x2c) + 4 +
                                      (*(int *)((int)this + 0x28) + -1) * 0x14);
              param_1[6].y = 1;
            }
          }
        }
        return 1;
      }
    }
    local_1c = local_1c + 1;
  } while( true );
}
