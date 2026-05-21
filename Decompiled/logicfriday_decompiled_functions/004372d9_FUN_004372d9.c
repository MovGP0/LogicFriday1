/* 004372d9 FUN_004372d9 */

void FUN_004372d9(HWND param_1,int param_2,int param_3)

{
  tagRECT local_28;
  int local_18;
  tagPOINT local_14;
  int local_c;
  HWND local_8;
  
  local_8 = GetDlgItem(param_1,param_2);
  GetWindowRect(local_8,&local_28);
  local_18 = local_28.bottom - local_28.top;
  local_c = local_28.right - local_28.left;
  local_14.x = local_28.left;
  local_14.y = local_28.top;
  ScreenToClient(param_1,&local_14);
  MoveWindow(local_8,local_14.x,local_14.y - param_3,local_c,local_18,1);
  return;
}
