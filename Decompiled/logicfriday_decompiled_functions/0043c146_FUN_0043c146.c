/* 0043c146 FUN_0043c146 */

undefined4 __thiscall FUN_0043c146(void *this,int param_1,int param_2,int param_3,int param_4)

{
  POINT pt;
  POINT pt_00;
  bool bVar1;
  BOOL BVar2;
  int iVar3;
  int local_8;
  
  *(undefined4 *)((int)this + 0x44) = 0;
  local_8 = 0;
  do {
    if (*(int *)((int)this + 0x28) + -1 <= local_8) {
      return *(undefined4 *)((int)this + 0x44);
    }
    bVar1 = false;
    pt.y = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14);
    pt.x = *(LONG *)(*(int *)((int)this + 0x2c) + local_8 * 0x14);
    BVar2 = PtInRect((RECT *)&param_1,pt);
    if ((BVar2 == 0) &&
       (iVar3 = (local_8 + 1) * 0x14, pt_00.y = *(LONG *)(*(int *)((int)this + 0x2c) + 4 + iVar3),
       pt_00.x = *(LONG *)(*(int *)((int)this + 0x2c) + iVar3),
       BVar2 = PtInRect((RECT *)&param_1,pt_00), BVar2 == 0)) {
      if (*(int *)(*(int *)((int)this + 0x2c) + 0xc + local_8 * 0x14) == 0) {
        if ((param_1 <= *(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c))) &&
           (*(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)) <= param_3)) {
          if ((*(int *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14) < param_2) &&
             (param_2 < *(int *)(*(int *)((int)this + 0x2c) + 4 + (local_8 + 1) * 0x14))) {
            bVar1 = true;
          }
          else if ((param_4 < *(int *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14)) &&
                  (*(int *)(*(int *)((int)this + 0x2c) + 4 + (local_8 + 1) * 0x14) < param_4)) {
            bVar1 = true;
          }
          goto LAB_0043c306;
        }
      }
      else if ((param_2 <= *(int *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14)) &&
              (*(int *)(*(int *)((int)this + 0x2c) + 4 + local_8 * 0x14) <= param_4)) {
        if ((*(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c)) < param_1) &&
           (param_1 < *(int *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c)))) {
          bVar1 = true;
        }
        else if ((param_3 < *(int *)(local_8 * 0x14 + *(int *)((int)this + 0x2c))) &&
                (*(int *)((local_8 + 1) * 0x14 + *(int *)((int)this + 0x2c)) < param_3)) {
          bVar1 = true;
        }
        goto LAB_0043c306;
      }
    }
    else {
      bVar1 = true;
LAB_0043c306:
      if (bVar1) {
        *(undefined4 *)((int)this + 0x44) = 1;
        *(undefined4 *)(*(int *)((int)this + 0x2c) + 8 + local_8 * 0x14) = 1;
      }
      else {
        *(undefined4 *)(*(int *)((int)this + 0x2c) + 8 + local_8 * 0x14) = 0;
      }
    }
    local_8 = local_8 + 1;
  } while( true );
}
