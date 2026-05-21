/* 00422a3c FUN_00422a3c */

undefined4 __fastcall FUN_00422a3c(int param_1)

{
  int iVar1;
  bool bVar2;
  bool bVar3;
  undefined4 uVar4;
  int local_34;
  int local_2c;
  int local_20;
  int local_14;
  int local_10;
  
  local_20 = 0;
  local_34 = 0;
  local_10 = 0;
  local_2c = 0;
  bVar2 = false;
  bVar3 = false;
  if ((*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f4) * 0x118) != 0) &&
     (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f0) * 0x118) == 0)) {
    for (local_14 = *(int *)(param_1 + 0x1654); local_14 < *(int *)(param_1 + 0x1658);
        local_14 = local_14 + 1) {
      if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 1) &&
         (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 3)) {
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 4;
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x28 + local_14 * 0xfc) = 0xffffffff;
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 1;
      }
    }
  }
  if ((*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x2300) * 0x118) != 0) &&
     (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22fc) * 0x118) == 0)) {
    for (local_14 = *(int *)(param_1 + 0x1654); local_14 < *(int *)(param_1 + 0x1658);
        local_14 = local_14 + 1) {
      if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 2) &&
         (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 3)) {
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 4;
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x28 + local_14 * 0xfc) = 0xfffffffe;
        *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 1;
      }
    }
  }
  if ((*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22ec) * 0x118) == 0) &&
     (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f8) * 0x118) == 0)) {
    for (local_14 = *(int *)(param_1 + 0x1654); local_14 < *(int *)(param_1 + 0x1658);
        local_14 = local_14 + 1) {
      if (*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 1) {
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc);
        if (iVar1 == 2) {
          bVar2 = true;
          local_20 = local_20 + 1;
        }
        else if ((iVar1 != 3) && (iVar1 == 4)) {
          local_34 = local_34 + 1;
        }
      }
      else if (*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 2) {
        iVar1 = *(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc);
        if (iVar1 == 2) {
          bVar3 = true;
          local_20 = local_20 + 1;
        }
        else if ((iVar1 != 3) && (iVar1 == 4)) {
          local_34 = local_34 + 1;
        }
      }
    }
    if ((bVar2) || (bVar3)) {
      if (bVar2) {
        if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f0) * 0x118) == 0) {
          local_2c = local_20;
          local_10 = 0;
        }
        else if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22f4) * 0x118) == 0) {
          local_2c = 0;
          local_10 = local_20;
        }
        else if ((local_34 == 1) || (local_34 % 2 != 0)) {
          local_2c = 1;
          local_10 = local_20 + -1;
        }
        for (local_14 = *(int *)(param_1 + 0x1654);
            (local_2c != 0 && (local_14 < *(int *)(param_1 + 0x1658))); local_14 = local_14 + 1) {
          if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 1) &&
             (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 2)) {
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 4;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x24 + local_14 * 0xfc) = 0xffffffff;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x28 + local_14 * 0xfc) = 0xffffffff;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 2;
            local_2c = local_2c + -1;
          }
        }
        for (; (local_10 != 0 && (local_14 < *(int *)(param_1 + 0x1658))); local_14 = local_14 + 1)
        {
          if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 1) &&
             (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 2)) {
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 3;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x24 + local_14 * 0xfc) = 0xffffffff;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 1;
            local_10 = local_10 + -1;
          }
        }
      }
      if (bVar3) {
        if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x22fc) * 0x118) == 0) {
          local_2c = local_20;
          local_10 = 0;
        }
        else if (*(int *)(param_1 + 0x3cc + *(int *)(param_1 + 0x2300) * 0x118) == 0) {
          local_2c = 0;
          local_10 = local_20;
        }
        else {
          local_10 = local_20;
          if (local_34 % 2 != 0) {
            local_2c = 1;
            local_10 = local_20 + -1;
          }
        }
        for (local_14 = *(int *)(param_1 + 0x1654);
            (local_2c != 0 && (local_14 < *(int *)(param_1 + 0x1658))); local_14 = local_14 + 1) {
          if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 2) &&
             (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 2)) {
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 4;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x24 + local_14 * 0xfc) = 0xfffffffe;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x28 + local_14 * 0xfc) = 0xfffffffe;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 2;
            local_2c = local_2c + -1;
          }
        }
        for (; (local_10 != 0 && (local_14 < *(int *)(param_1 + 0x1658))); local_14 = local_14 + 1)
        {
          if ((*(int *)(local_14 * 0xfc + *(int *)(param_1 + 0x3a4)) == 2) &&
             (*(int *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) == 2)) {
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x18 + local_14 * 0xfc) = 3;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0x24 + local_14 * 0xfc) = 0xfffffffe;
            *(undefined4 *)(*(int *)(param_1 + 0x3a4) + 0xbc + local_14 * 0xfc) = 1;
            local_10 = local_10 + -1;
          }
        }
      }
      if ((local_10 == 0) && (local_2c == 0)) {
        uVar4 = 0;
      }
      else {
        uVar4 = 0x230001;
      }
    }
    else {
      uVar4 = 0;
    }
  }
  else {
    uVar4 = 0;
  }
  return uVar4;
}
