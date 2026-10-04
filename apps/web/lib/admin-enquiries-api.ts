import { adminApiJson } from "@/lib/admin-api";
import {
  ENQUIRIES_PAGE_SIZE,
  type CateringEnquiryDetail,
  type CateringEnquiryList,
  type ContactMessageDetail,
  type ContactMessageList,
} from "@/lib/admin-enquiries";

function pageParams(page = 1) {
  return new URLSearchParams({
    limit: String(ENQUIRIES_PAGE_SIZE),
    offset: String((page - 1) * ENQUIRIES_PAGE_SIZE),
  });
}

export function getAdminCateringEnquiries(page = 1): Promise<CateringEnquiryList> {
  return adminApiJson<CateringEnquiryList>(`/api/admin/catering-enquiries?${pageParams(page)}`);
}

export function getAdminCateringEnquiry(id: number): Promise<CateringEnquiryDetail> {
  return adminApiJson<CateringEnquiryDetail>(`/api/admin/catering-enquiries/${id}`);
}

export function getAdminContactMessages(page = 1): Promise<ContactMessageList> {
  return adminApiJson<ContactMessageList>(`/api/admin/contact-messages?${pageParams(page)}`);
}

export function getAdminContactMessage(id: number): Promise<ContactMessageDetail> {
  return adminApiJson<ContactMessageDetail>(`/api/admin/contact-messages/${id}`);
}
