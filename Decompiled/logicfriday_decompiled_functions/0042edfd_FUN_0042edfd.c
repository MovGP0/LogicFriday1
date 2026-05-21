/* 0042edfd FUN_0042edfd */

undefined4 __thiscall
FUN_0042edfd(void *this,LONG param_1,LONG param_2,undefined4 *param_3,LONG *param_4,POINT *param_5)

{
  LONG LVar1;
  int iVar2;
  POINT pt;
  POINT pt_00;
  BOOL BVar3;
  int iVar4;
  int local_2c;
  tagRECT local_18;
  int local_8;
  
  if (*(int *)((int)this + 0x16c4) != 0) {
    for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c4); local_8 = local_8 + 1) {
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x48) == 0) {
        if (**(int **)(*(int *)((int)this + 0x16cc) + local_8 * 4) != 9) {
          local_18.left = *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xac);
          local_18.top = *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xb0);
          local_18.right = local_18.left;
          local_18.bottom = local_18.top;
          InflateRect(&local_18,6,6);
          pt.y = param_2;
          pt.x = param_1;
          BVar3 = PtInRect(&local_18,pt);
          if (BVar3 != 0) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe0) == -3) {
              iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
              iVar2 = *(int *)(iVar4 + 0xac);
              iVar4 = *(int *)(iVar4 + 0xb0);
              *param_3 = 1;
              param_3[1] = local_8;
              param_3[0xe] = local_8;
              FUN_0043ac51(param_3,iVar2,iVar4);
              return 1;
            }
            param_5[3].x = *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe0);
            iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
            LVar1 = *(LONG *)(iVar4 + 0xb0);
            param_5->x = *(LONG *)(iVar4 + 0xac);
            param_5->y = LVar1;
            FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4),param_5);
            *param_3 = 2;
            param_3[3] = param_5[3].x;
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4) + 0x38) != -3) {
              param_3[0xe] = *(undefined4 *)
                              (*(int *)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4) + 0x38);
            }
            FUN_0043ac51(param_3,param_5[1].x,param_5[1].y);
            *param_4 = param_5[2].x;
            return 1;
          }
        }
        for (local_2c = 0;
            local_2c < *(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x18);
            local_2c = local_2c + 1) {
          local_18.left =
               *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x6c + local_2c * 8)
          ;
          local_18.top = *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0x70 +
                                  local_2c * 8);
          local_18.right = local_18.left;
          local_18.bottom = local_18.top;
          InflateRect(&local_18,6,6);
          pt_00.y = param_2;
          pt_00.x = param_1;
          BVar3 = PtInRect(&local_18,pt_00);
          if (BVar3 != 0) {
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe4 + local_2c * 4)
                == -3) {
              iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
              iVar2 = *(int *)(iVar4 + 0x6c + local_2c * 8);
              iVar4 = *(int *)(iVar4 + 0x70 + local_2c * 8);
              *param_3 = 0;
              param_3[1] = local_8;
              param_3[2] = local_2c;
              FUN_0043ac51(param_3,iVar2,iVar4);
              return 1;
            }
            param_5[3].x = *(LONG *)(*(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4) + 0xe4 +
                                    local_2c * 4);
            iVar4 = *(int *)(*(int *)((int)this + 0x16cc) + local_8 * 4);
            LVar1 = *(LONG *)(iVar4 + 0x70 + local_2c * 8);
            param_5->x = *(LONG *)(iVar4 + 0x6c + local_2c * 8);
            param_5->y = LVar1;
            FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4),param_5);
            *param_3 = 2;
            param_3[3] = param_5[3].x;
            if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4) + 0x38) != -3) {
              param_3[0xe] = *(undefined4 *)
                              (*(int *)(*(int *)((int)this + 0x16d0) + param_5[3].x * 4) + 0x38);
            }
            FUN_0043ac51(param_3,param_5[1].x,param_5[1].y);
            *param_4 = param_5[2].x;
            return 1;
          }
        }
      }
    }
    param_5->x = param_1;
    param_5->y = param_2;
    for (local_8 = 0; local_8 < *(int *)((int)this + 0x16c8); local_8 = local_8 + 1) {
      if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x40) == 0) {
        param_5[3].x = local_8;
        iVar4 = FUN_0043bad3(*(void **)(*(int *)((int)this + 0x16d0) + local_8 * 4),param_5);
        if (iVar4 != 0) {
          *param_3 = 2;
          param_3[3] = local_8;
          if (*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x38) != -3) {
            param_3[0xe] = *(undefined4 *)
                            (*(int *)(*(int *)((int)this + 0x16d0) + local_8 * 4) + 0x38);
          }
          FUN_0043ac51(param_3,param_5[1].x,param_5[1].y);
          *param_4 = param_5[2].x;
          return 1;
        }
      }
    }
  }
  return 0;
}
