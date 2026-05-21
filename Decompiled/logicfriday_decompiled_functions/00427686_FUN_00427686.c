/* 00427686 FUN_00427686 */

undefined4 __thiscall
FUN_00427686(void *this,HDC param_1,int param_2,int param_3,int param_4,int param_5,int param_6)

{
  int iVar1;
  bool bVar2;
  undefined4 uVar3;
  int iVar4;
  int x;
  int local_38;
  int local_24;
  int local_1c;
  int local_18;
  int local_14;
  int local_10;
  int local_c;
  int local_8;
  
  if (*(int *)(*(int *)((int)this + 0x3a4) + 0xb8 + param_2 * 0xfc) == 0) {
    if (*(int *)(*(int *)((int)this + 0x3a4) + 0x3c + param_2 * 0xfc) != param_6) {
      bVar2 = true;
      local_38 = *(int *)(*(int *)((int)this + 0x3a4) + 0x3c + param_2 * 0xfc);
      do {
        local_38 = local_38 + 1;
        if (param_6 < local_38) goto LAB_00427736;
      } while (*(int *)(local_38 * 0x48 +
                       *(int *)(*(int *)((int)this + 0x2678) +
                               *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_2 * 0xfc) * 4))
               == -1);
      bVar2 = false;
LAB_00427736:
      if (!bVar2) {
        return 0;
      }
    }
    if (*(int *)(*(int *)((int)this + 0x3a4) + 0xb0 + param_2 * 0xfc) == param_4) {
      local_c = param_3;
      local_8 = param_4;
      MoveToEx(param_1,param_3,param_4,(LPPOINT)0x0);
      iVar4 = *(int *)(*(int *)((int)this + 0x3a4) + 0xac + param_2 * 0xfc);
      iVar1 = *(int *)(*(int *)((int)this + 0x3a4) + 0xb0 + param_2 * 0xfc);
      LineTo(param_1,iVar4,iVar1);
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   param_3,param_4);
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   iVar4,iVar1);
      *(undefined4 *)
       (*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x14) = 1;
      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x18
              ) = param_2;
      *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xb8 + param_2 * 0xfc) = 1;
      *(int *)(*(int *)((int)this + 0x3a4) + 0xe0 + param_2 * 0xfc) =
           *(int *)((int)this + 0x16c8) + -1;
      uVar3 = 1;
    }
    else {
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   param_3,param_4);
      local_c = param_3;
      local_8 = param_4;
      MoveToEx(param_1,param_3,param_4,(LPPOINT)0x0);
      iVar4 = FUN_00428eb1(this,param_5,
                           *(int *)(*(int *)((int)this + 0x3a4) + 0x40 + param_2 * 0xfc),param_6,1,1
                          );
      iVar1 = local_8;
      x = local_c + iVar4 * -0xf;
      LineTo(param_1,x,local_8);
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   x,iVar1);
      iVar4 = *(int *)(*(int *)((int)this + 0x3a4) + 0xb0 + param_2 * 0xfc);
      local_c = x;
      local_8 = iVar1;
      LineTo(param_1,x,iVar4);
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   x,iVar4);
      iVar1 = *(int *)(*(int *)((int)this + 0x3a4) + 0xac + param_2 * 0xfc);
      local_c = x;
      local_8 = iVar4;
      LineTo(param_1,iVar1,iVar4);
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   iVar1,iVar4);
      *(undefined4 *)
       (*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x14) = 1;
      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x18
              ) = param_2;
      *(undefined4 *)(*(int *)((int)this + 0x3a4) + 0xb8 + param_2 * 0xfc) = 1;
      *(int *)(*(int *)((int)this + 0x3a4) + 0xe0 + param_2 * 0xfc) =
           *(int *)((int)this + 0x16c8) + -1;
      uVar3 = 1;
    }
  }
  else {
    iVar4 = FUN_00428837(this,param_2,param_3,param_4,param_5,param_6,&local_1c,&local_14);
    if (iVar4 == 0) {
      uVar3 = 0;
    }
    else {
      FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4),
                   param_3,param_4);
      local_c = param_3;
      local_8 = param_4;
      MoveToEx(param_1,param_3,param_4,(LPPOINT)0x0);
      if (local_18 == local_8) {
        local_24 = local_18;
        LineTo(param_1,local_1c,local_18);
        FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4
                               ),local_1c,local_18);
      }
      else {
        LineTo(param_1,local_1c,param_4);
        FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4
                               ),local_1c,param_4);
        local_c = local_1c;
        local_8 = param_4;
        local_24 = local_18;
        LineTo(param_1,local_1c,local_18);
        FUN_0043ac51(*(void **)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4
                               ),local_1c,local_18);
      }
      FUN_004287c6(this,param_1,local_1c,local_24);
      *(undefined4 *)
       (*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x14) = 2;
      *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4) + 0x20
              ) = local_14;
      iVar4 = *(int *)(*(int *)(*(int *)((int)this + 0x16d0) + -4 + *(int *)((int)this + 0x16c8) * 4
                               ) + 0x28);
      local_10 = iVar4 + -1;
      FUN_0043ae30(*(void **)(*(int *)((int)this + 0x16d0) + local_14 * 4),local_1c,local_24,
                   *(int *)((int)this + 0x16c8) + -1,local_10,
                   *(int *)(*(int *)(*(int *)(*(int *)((int)this + 0x16d0) + -4 +
                                             *(int *)((int)this + 0x16c8) * 4) + 0x2c) + 0xc +
                           (iVar4 + -2) * 0x14));
      uVar3 = 1;
    }
  }
  return uVar3;
}
